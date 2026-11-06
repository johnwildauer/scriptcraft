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
  - **AC-WRITING-002.9:** Desktop editing shall allocate and retain complete rendered page geometry
    for scroll extent, visible-page status, pointer hit testing, selection, zoom, and conditional
    caret reveal, while painting only pages intersecting the actual editor clip plus at most one
    rendered page of fixed overscan before and after that visible range. Pages outside that paint
    range shall not submit page fills, lines, selection, text, underlines, caret, or IME preedit.
    Viewport culling shall not change fixed page dimensions, source mapping, page count, interaction
    ownership, same-frame input scheduling, or the whole-layout object on scroll; it makes no timing
    guarantee.
  - **AC-WRITING-002.10:** Desktop text painting shall batch each visible nonempty screenplay line
    into a galley, splitting only at effective text-color boundaries such as a selection span.
    Every display scalar shall retain its deterministic fixed-cell x position from document layout
    geometry, independent of host-font advances or kerning; bundled role faces, fallback glyphs,
    selected/unselected colors, subdued hidden-text color, underline geometry, and presentation zoom
    shall remain intact. The core layout's document-point pagination and caret/hit-test geometry
    remain authoritative. Galleys shall preserve the line's display text and use the layout's
    display/source map for selection boundaries, including Unicode casing expansions; canonical
    UTF-8 text, source offsets, page rectangles, selection endpoints, and editing behavior shall not
    change. Only painted pages may build or reuse galleys, and cache identity shall include all
    display, font, fixed-cell, color, and scale inputs that affect the span. Diagnostics may report
    only numeric galley build/reuse counts and other value-free categories; this requirement makes no
    timing guarantee.
  - **AC-WRITING-002.11:** A layout revision may reuse immutable per-element wrap values only when
    stable element identity, complete canonical content, role, layout purpose, title/body context,
    effective paragraph geometry, effective text style, and wrapping limits match. A fingerprint
    shall not authorize reuse without collision-safe canonical-content verification. Reuse shall
    preserve canonical/display text, Unicode display/source maps, deterministic document-point
    geometry, and complete page output equivalent to a fresh layout for the same project and
    effective inputs. Whole-document page placement and keep-with-next handling shall still run for
    every layout revision, preserving explicit page starts, title pages, empty paragraphs, and
    oversized elements. Retention shall be bounded and invalidated or rekeyed safely for changed
    source/runtime inputs, purpose changes, document replacement, undo/redo, and structural edits.
    Presentation scale/palette and egui font-atlas or context-local galley data shall not enter the
    core wrap cache. Structural counters may report wraps built and reused; no timing improvement or
    fully incremental pagination guarantee is implied.
  - **AC-WRITING-002.12:** Derived interaction indexes over a complete layout shall preserve its
    existing page/line geometry, source/display mapping, caret boundaries, selection behavior, and
    deterministic document-order tie-breaking. Caret and owner-page lookup shall be restricted to
    the requested element's placed lines; nearest body-line hit testing shall use ordered vertical
    candidates; vertical movement shall address the current and adjacent body lines without
    flattening the complete document line set. Title pages shall remain excluded from body
    hit-testing and navigation. The indexes shall be rebuilt with each fresh complete layout and
    reused only with that immutable layout, and shall not contain desktop presentation state.
    Value-free diagnostics may report indexed query calls and index entries examined; no timing
    improvement is promised.
  - **AC-WRITING-002.13:** Ordinary single-paragraph text and role edits may use change-scoped
    validation only when the editing session holds a complete-valid baseline, the selected semantic
    identity and managed-source index entry are unique and unchanged, every non-target body element
    and the full title page remain exactly equal to the baseline, the candidate remains
    single-element and structurally unchanged, the affected semantic element and source projection
    validate locally, every source-tree node outside the uniquely mapped paragraph remains equal
    to that baseline, and global source identity/reference inputs remain equal to that baseline.
    Any missing, stale, ambiguous, structurally changed, or otherwise unprovable precondition shall
    use complete validation or reject the detached candidate; it shall never weaken validation.
    Import/session creation, structural/global edits, undo/redo restoration, derived publication,
    document replacement, explicit public validation, standalone layout builders, and native
    Save/Save As retain complete validation. A layout requested from a live validated session may
    trust that session boundary; arbitrary project/layout callers may not. Candidate publication,
    UTF-8-safe selection, undo/redo, FDX opaque-data preservation, dirty/no-op semantics, and
    save-failure safety remain transactional and unchanged. Structural counters may report index
    rebuilds/lookups, scoped accepts, complete candidate validations, and safe fallbacks; no timing
    guarantee or speed result is implied.
  - **AC-WRITING-002.14:** Desktop page-edge breathing room shall belong to scrollable content, move
    with that content, and contribute to its scroll extent rather than remain as a fixed viewport
    inset. Script View shall own independent top and bottom content-edge spacing so the first page
    can begin flush with the viewport while the document end retains its own allowance. Script View
    page content shall remain unobscured by scroll-edge fade overlays during ordinary vertical or
    horizontal scrolling; this surface-local presentation policy shall not change unrelated scroll
    areas. Presentation spacing shall not alter core document layout, and actual page allocation and
    prepaint zoom compensation shall use equivalent asymmetric page origins and content extents while
    scaling page-local coordinates independently from fixed presentation spacing. The desktop
    toolbar's visible lower edge and returned panel boundary shall coincide: its explicit outer
    composition shall not acquire implicit vertical gaps or child-response overflow, and the Script
    View viewport shall begin at that shared boundary. Closed Elements control/caption allocation
    shall suppress inherited post-control vertical spacing before the ComboBox is allocated, while
    popup role rows and nested toolbar controls retain their normal spacing.

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
    DocumentRef, Content, WindowState, TextState, then the 17 ElementSettings blocks in directed role
    order, then TitlePage when present. Generated root TextState has Scaling, Selection, and
    ShowInvisibles attributes in that order. A generated DocumentRef carries one current save
    timestamp and a fresh UUID v4 ID.
    TextState is disposable root-only view state: Open projects only one direct FinalDraft child,
    defaults invalid/missing Scaling to 100 before 75–300% normalization, and rejects a complete
    malformed/unmappable Selection to caret 0,0. Its ordered Selection uses the bounded ScriptCraft
    UTF-16 flat-offset mapping over canonical body text with one paragraph separator, mapped to
    editor UTF-8 byte positions. Open applies zoom and Selection once; transactional Save/Save As
    overwrite the direct root block with current zoom and ordered Selection plus ShowInvisibles No.
    Nested TitlePage TextState is source-preserved and never projected into the screenplay editor.
    Zoom and caret movement do not alter Project semantics, dirty state, history, fixed layout, or
    exports.
    Serialized output shall begin with exactly one reference-compatible UTF-8 standalone XML
    declaration; this lexical declaration is emitted once on each save/re-serialization. Newly
    generated preservation-free output uses the canonical declaration followed by one LF. For
    source-backed output, the declaration itself adds no trailing LF: retained top-level nodes
    provide the declaration-to-root separator unchanged, including whitespace or its absence, so
    repeated saves cannot accumulate separator characters. The generated section order shall not
    reorder imported top-level or opaque source content.
    Newly generated ScriptCraft structure shall use LF line endings, two ASCII spaces per markup
    indentation level, blank lines between direct FinalDraft children and adjacent ElementSettings,
    no blank lines between Content paragraphs, and one final LF. Generated whitespace may be added
    only in element-only generated structure. Newly inserted or split managed Content paragraphs
    shall receive LF/two-space sibling indentation as a best-effort cosmetic normalization only when
    the Content child sequence is unambiguously Paragraph elements plus structural whitespace and no
    effective `xml:space="preserve"` applies. Any detached structural-whitespace normalization shall
    be applied equally to the expected source-tree comparison; re-read integrity checks remain strict.
    Canonical nonempty Text content, including whitespace-only values, tabs, LF and CR, and CDATA,
    comments, processing instructions, mixed content, namespaces, unknown imported structures, and
    source order shall not be rewritten. For a managed screenplay body or direct managed title-role
    Paragraph whose canonical text is empty, the semantic run projection and direct managed `Text`
    children shall both be empty; retain the Paragraph identity, role, source Type, direct Paragraph
    attributes, attachments, and non-Text source-owned children. Empty source `Text` runs (including
    formatted empty runs) project no semantic runs and are removed only during managed writer
    reconciliation. Whitespace-only canonical text is nonempty and retains its Text representation.
    Expected/actual source-tree comparison omits serializer-invisible empty XML text nodes before
    merging adjacent nonempty text nodes; it does not omit empty CDATA, whitespace-only text, empty
    elements, comments, processing instructions, or opaque empty `Text` controls. Ambiguous
    mixed-content and `xml:space` contexts are
    preserved conservatively; imported opaque structures are never globally pretty-printed. When
    cosmetic normalization is unsafe or ambiguous, the reconciled managed paragraph shall be emitted
    at its source-tree position without adding sibling formatting; missing beautification shall not
    refuse an otherwise valid save. This formatting boundary does not promise lexical identity or
    blanket Final Draft compatibility. After supported managed-scene reconciliation succeeds,
    generated `SceneProperties` and `SceneArcBeats` element-only containers shall use deterministic LF
    and two-space indentation computed from the managed heading's actual depth in the preserved source
    tree. The same structural whitespace shall be present in the detached expected tree before strict
    code 30 comparison; that comparison shall not special-case SceneProperties. Deferred/ambiguous
    scene subtrees, effective `xml:space` preservation contexts, mixed or opaque content, CDATA,
    comments, processing instructions, and arc Paragraph Text/runs shall not receive cosmetic
    whitespace. Empty `SceneArcBeats` remains empty and self-closing. This bounded generated-container
    rule does not authorize global pretty-printing of imported FDX.
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
    prior identity snapshot. Empty managed screenplay paragraphs remain Paragraphs without direct
    managed `Text` children through Save, Save As, reopen, and repeated serialization; clearing and
    retyping synchronizes semantic runs and managed source runs transactionally.
  - **AC-WRITING-003.5:** Every supported managed Scene Heading shall contain generated
    `SceneProperties` and `SceneArcBeats` before its first managed `Text` child. Generated
    `SceneProperties` uses Length `1/8`, Page `1`, and empty Title; supported imported scenes
    regenerate those non-arc attributes and children on derived refresh, without recalculating Page
    or Length. New-document and imported-scene generation use the same defaults and heading
    namespace. At most one same-namespace `SceneProperties` and one same-namespace `SceneArcBeats`
    may be reconciled; duplicate or namespace-conflicting containers defer that scene. For new
    documents, scene-cast arcs project Character elements until the next heading: trim outer
    Unicode whitespace, remove one recognized trailing delivery extension ((V.O.), (O.S.), (O.C.),
    (CONT'D), (SUBTITLE), (TEXT), or (V.O)), ignore empty results, deduplicate by Unicode lowercase
    association key in first-occurrence order, and store the derived name in Unicode uppercase.
    Canonical Character text remains unchanged. This normalized projection applies to new documents
    and supported imported scenes. Supported imported entries with one nested Paragraph and an
    ordered sequence of same-namespace `Text` runs containing only text or CDATA are app-owned
    derived membership after deserialization; whitespace-only formatting nodes between runs are
    allowed. The nested Paragraph remains opaque to screenplay content and retains its complete
    supported subtree, including source attributes, current ID, ordered run nodes, text, and run
    formatting. Match entries by normalized Character name, update only the derived beat Name/order,
    add missing names with fresh UUID v4 nested IDs, and remove unmatched names without requiring
    ScriptCraft creation history. A retained Character keeps its nested arc content through unrelated
    Character additions, removals, reordering, and cue-case changes. Removing a Character removes its
    complete derived arc Paragraph and text; renaming to a different normalized key is removal plus a
    new blank arc, never a transfer by position or nested ID. Removal still requires a unique
    registered paragraph ID without known references or external source-tree uses. Duplicate beats,
    multiple nested Paragraphs, unknown non-text child structures, and unsafe references defer only
    that scene's derived update: its complete original SceneProperties/SceneArcBeats subtree and Cast
    Lists remain unchanged while independent supported scenes may reconcile. This safe-stop scopes
    the derived update, not an otherwise valid canonical native Save or Save As. Model/preservation
    validation, managed-source synchronization, required identity allocation, serialization/re-read
    comparison, conflict detection, and atomic publication remain strict and fatal on failure.
    An otherwise valid explicit cross-paragraph selection deletion may remove a Scene Heading with
    its complete recognized generated SceneProperties/SceneArcBeats subtree only when the heading
    and every supported nested arc Paragraph have unique registered identities with no known
    references, foreign anchors, duplicate source definitions, or external source uses. This is a
    deletion-only ownership rule: unknown/ambiguous children and unsafe identities still reject the
    candidate, while backspace/join, split, and role transitions retain their existing attachment
    safe-stops. Cast Lists, unrelated source nodes, and unsupported authored arc data are never
    removed by this allowance. Existing Scene Heading split/join attachment safe-stops remain
    unchanged. Once a supported scene
    candidate has been validated, its generated SceneProperties/SceneArcBeats element-only structure
    receives LF/two-space formatting at actual source-tree depth; deferred scene subtrees remain
    untouched. Effective xml:space preservation and mixed/opaque child contexts skip cosmetic
    formatting without blocking Save. Retained arc Paragraph IDs, attributes, ordered Text runs, and
    content remain untouched, and strict expected-tree comparison includes the generated whitespace.
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
    right paragraph; joining or deleting removes only the removed paragraph's ID, with a Scene
    Heading's recognized generated arc Paragraph IDs removed only by the bounded, safe explicit
    cross-paragraph selection-deletion rule in AC-WRITING-003.5. Undo/redo within
    the active session restores the exact identity state in its history snapshots. Content recreated
    after deletion receives a new allocation, but with no historical tombstone registry the
     allocator cannot guarantee uniqueness against every ID deleted in the past. Matching normalized
     scene-arc names retain their nested Paragraph ID and complete supported text/run subtree; new or
     renamed names receive fresh allocations and removed entries in the supported derived shape lose
     their IDs and arc text after reference-safety checks, including when the document was separately
     reopened. Nested ID shape or ScriptCraft creation history is not used as character-association or
     deletion-ownership evidence. Unsupported arc payloads still stop reconciliation atomically and
     remain source-preserved. Opaque TitlePage Paragraphs that are not managed title roles remain
     source-tree occurrences: a distinct occurrence retains its existing global Paragraph identity,
     while repeated occurrences of the same raw ID are excluded from the global identity registry and
     preserve each element, order, attributes, content, and ID verbatim. Repeated opaque occurrences
     may not collide with a globally registered managed or otherwise referenced Paragraph identity;
      duplicate checks remain strict for managed body/title roles and identity consumers.
    Final Draft's observed behavior of regenerating nested scene-arc Paragraph UUIDs on every save is
    intentionally not mirrored: matched normalized Character names retain their IDs in ScriptCraft.
    Revisit this difference only if concrete interoperability evidence shows regeneration is required
    for semantics, references, or reopen behavior.
  - **AC-WRITING-003.7:** A native FDX root DocumentRef is the durable current-save record. Preserve
    its existing ID verbatim across Save and Save As; every successful transactional Save and Save As
    writes one current local-wall-clock DateTime in the exact `YYYYMMDDTHHMMSS` shape (falling back
    to UTC only when local offset discovery fails). Capture a separate session-open wall-clock time
    when a document is deserialized into an editing session or a new session starts; this ephemeral
    value is independent of DocumentRef DateTime, remains fixed across saves/rebases, and is outside
    canonical content and undo history. On ordinary Save, append exactly one direct DocumentRef child
    XRef with that session-open timestamp and a fresh collision-checked lowercase UUID v4 only when
    there are no direct-child XRefs; otherwise preserve all XRefs and append none. Every Save As
    appends exactly one such XRef regardless of existing history. On every successful Save or Save
    As, update DocumentRef DateTime to the independently captured current saving time and preserve its
    existing ID. Session-open and saving times may coincide at FDX's one-second resolution; do not
    fabricate monotonic timestamps. Existing XRefs and unrelated DocumentRef source data remain
    unchanged. A missing DocumentRef is generated before Content with DateTime then id attributes; a
    present DocumentRef retains its ID and source position while only its recognized DateTime value is
    updated (or added without normalizing unrelated attributes). Direct source-backed serialization
    has no lineage side effects. The authorized DocumentRef/XRef delta is reconciled only in the
    detached save candidate after original code 4 graph comparison and before strict code 29 identity
    and code 30 source-tree checks. Any preparation, codec, conflict, or publication failure leaves
    destination, live lineage tree, session-open timestamp, path, baseline, selection, and dirty state
    unchanged.
  - **AC-WRITING-003.8:** Root WindowState is disposable native desktop view state, not screenplay
    semantics or session history. Open projects only one unambiguous direct FinalDraft/WindowState
    in the root element's expanded namespace; nested lookalikes are ignored. Left and Top accept
    bounded signed integers, Width and Height require bounded positive integers, and malformed,
    overflowing, missing, duplicate, or otherwise unusable root state shall not block screenplay
    Open or trigger a restore. Unknown/missing Mode uses Normal. A valid imported geometry is sent
    once as a best-effort native outer position and inner logical size after activation; absent or
    unusable geometry leaves the current window unchanged, and no later frame retries the request.
    Fresh-process native startup and generated/fallback FDX WindowState use one shared typed
    default, mapping Left/Top to outer position and Width/Height to inner logical size. Save/Save As
    serialize the latest valid OS-logical outer position and inner size, using that same default only
    when no complete native snapshot is available.
    Window movement/resizing does not alter Project semantics, revision, dirty state, history, fixed
    layout, or PDF output. Actual transactional Save/Save As replaces all direct root WindowState
    duplicates with one canonical Height, Left, Mode, Top, Width block, inserting a missing block
    after PageLayout or before ElementSettings; preservation-free output places it after Content.
    Direct source-backed serialization leaves imported WindowState untouched. TitlePage, nested
    lookalikes, Content, DocumentRef/XRef, ElementSettings, and unrelated opaque source remain
    unchanged. WindowState is reconciled only in the detached save candidate and participates in the
    expected tree before strict source-tree comparison; failed saves publish neither its replacement
    nor other candidate changes.

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
  internal updater consumes the uppercase derived scene-arc projection after each Scene Heading to
  the next heading or end, retaining case-insensitive alphabetical sorting and parenthesized
  formatting, and periodically updates safely correctable managed lists.
  - **AC-WRITING-009.1:** Direct edits may be silently corrected on refresh; Save and Save As check
    and silently correct the detached candidate before emitting it. Stored derived Cast List text
    is uppercase; canonical Character cues retain their typed casing.
  - **AC-WRITING-009.2:** A deferred unsupported scene leaves its entire SceneProperties/SceneArcBeats
    subtree and all Cast Lists unchanged, while other supported scenes may update. A formatted Cast
    List that cannot be corrected without rewriting its runs keeps its text and run attributes;
    neither limitation blocks an otherwise valid canonical native Save or Save As.
  - **AC-WRITING-009.3:** After successful save, the live document and clean baseline match the
    published candidate; failed preparation, conflict or replacement publishes no candidate
    correction. Refresh, undo/redo correction, and pending saves use the same per-scene deferral and
    remain transactional and deterministic. Genuine model, managed-source, identity, preservation,
    re-read, conflict, and storage failures remain fatal.
  - **AC-WRITING-009.4:** Desktop derived Cast List and supported scene metadata refresh shall retain
    the existing one-second maximum scheduling cadence when a relevant project revision is pending,
    while due ticks for an already successfully attempted revision shall skip project cloning and
    reconciliation. The first active-document refresh shall occur when required. Content edits,
    undo/redo, successful derived publication, successful Save/Save As candidate publication, and
    document replacement shall update or invalidate the scheduling watermark so work is neither
    repeated for one unchanged revision nor left pending after a relevant change. Accepted editor
    input already queued for the frame shall be dispatched before scheduled refresh; a pending FDX
    migration modal shall continue to exclude both. Save/Save As shall prepare the complete current
    derived candidate independently of timer state. A fatal refresh failure shall retain its pending
    scope and failure category, but suspend further timer/direct refresh attempts until a later
    relevant invalidation reactivates that scope; unrelated edits and due ticks shall not retry it. A
    relevant scoped invalidation may reactivate only intersecting failed scenes; a complete
    invalidation may reactivate failed complete work. Fatal failures remain distinct from safe-stop
    deferrals: deferred work remains pending but is not retried unchanged and is reconsidered after a
    later relevant revision, while fatal correction, validation, preservation, re-read, conflict, and
    storage failures are not reported as successful deferrals. Value-free diagnostics may distinguish
    attempted, completed, skipped, changed, unchanged, deferred, and failed outcomes, including
    pending/retryable/deferred/failed summaries. This scheduling contract makes no timing guarantee
    and does not alter scene metadata, source preservation, save safety, modal behavior, or interaction
    semantics.

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
    default Item order leading Flexible Space, New, Open, Save, Save As, pre-Elements Flexible
    Space, Elements, post-Elements Flexible Space, Show/Hide, Export Text, Export PDF, and trailing
    Flexible Space. The four Flexible Spaces shall have unique identities and divide the post-budget
    remainder equally when every ordinary Item fits. A trailing outlined Double Arrows Material
    overflow indicator shall be present in the catalog but rendered only when non-spacing Items
    overflow; it shall consume no permanent width and shall have no visible caption.
  - **AC-WRITING-005.2:** Given an actionable toolbar Item is activated, the desktop shall route it
    through application/session adapters and preserve command safety, status, persistence, and
    fixed-layout PDF behavior. New shall not replace a dirty session without the writer's
    explicit informed consent, without prescribing a particular dialog or action sequence.
  - **AC-WRITING-005.3:** Toolbar Items shall resolve their declarative graphic, caption visibility,
    and enabled/disabled foreground through desktop toolbar presentation. Action Items shall use an
    icon-first, borderless icon control with a small explicit padding region as the hover/click
    surface; that region shall support independently adjustable left, top, right, and bottom padding.
    The centered text caption beneath an action graphic shall remain outside that response,
    retain its resolved normal foreground when enabled, remain available for accessibility use, and
    not add a redundant visible-label tooltip. Action and Elements captions shall share an explicit
    control-to-caption vertical rhythm and fit within the common Item-group budget. Both captions
    shall use the same group-relative rectangle derived from the nominal control slot and shared gap,
    and shall be centered horizontally and vertically within it. Elements shall own a separate
    vertical optical adjustment with a neutral default; its nominal caption placement shall not
    depend on the closed ComboBox response height. The complete composition shall remain contained
    by the toolbar panel. Icon graphics shall use their assigned
    typed Material icons; graphic, behavior, enablement, and caption visibility shall remain separate
    item fields. Toolbar and status Material glyphs shall use the bundled native Material Symbols
    Rounded TTF, with outlined and filled forms selected through the corresponding `FILL` axis.
    Toolbar icons may override supported axes per icon, while status icons retain their defaults.
    These axes shall be explicit presentation metadata and shall not affect text/document fonts.
    Show/Hide shall display outlined Visibility and independently specified outlined Keyboard Arrow
    Down graphics plus its caption, and Export Text shall display its outlined File Export graphic
    plus its caption. Both shall use the disabled foreground for every graphic and caption, with no
    hover fill or foreground change, pointer or keyboard activation, focus acquisition, popup,
    command dispatch, or accessibility state that presents either Item as enabled. If either Item
    appears in overflow, its label shall retain the disabled foreground and remain noninteractive.
  - **AC-WRITING-005.4:** Given the current caret/focus is within a screenplay element, the Elements
    Item shall display that element's explicit role using the supported labels in
    REQ-WRITING-LOOP-007. The central Elements ComboBox's closed control shall use an outlined
    Unfold More Material icon instead of egui's default triangle, while preserving the ComboBox's
    selection, keyboard, popup, and accessibility behavior. The central dropdown and the Elements
    submenu in toolbar overflow shall
    derive their desired vertical extent from all supported role rows and active menu metrics,
    including row spacing and popup frame allowance, rather than an arbitrary short fixed height.
    The Elements closed selected role and role rows in the central dropdown and overflow submenu
    shall use the same separately named text-size default. The Elements caption shall retain toolbar
    item text sizing and use the centered nominal caption rectangle, independently of the stock
    ComboBox response height; its separately owned vertical optical adjustment shall not move action
    captions. Role-option rows shall retain their separately owned horizontal inset. Popup
    desired-height measurement shall use the role-row font metrics.
    All supported labels shall be simultaneously visible at ordinary usable viewport heights;
    viewport bounds are the only height limitation, and constrained viewports shall retain bounded
    scrolling that makes every role reachable. Choosing a label from either presentation shall apply
    the role through the transactional session operation and preserve the document on rejection.
  - **AC-WRITING-005.5:** Given a toolbar width, Space shall consume its deterministic catalog
    width, rendered normal Items shall consume their complete graphic arrangement and visible
    caption width measured at runtime using active egui style/font metrics and resolved icon point
    sizes, and the four Flexible Space Items shall divide the remaining width equally after the
    shared horizontal row-padding and inter-item-gap budget whenever all ordinary Items fit. At
    narrow widths Flexible Spaces shall be excluded from overflow and consume no
    allocation slots. The toolbar shall retain leftmost fitting non-spacing Items and expose the
    remaining non-spacing Items through a far-right menu whose icon-only trigger is the catalog's
    outlined Double Arrows Material graphic, uses the accessible name Toolbar overflow, and is
    rendered only when entries overflow. Disabled Show/Hide and Export Text entries in that menu
    shall remain disabled-colored, noninteractive labels; enabled action entries and Elements role
    choices shall remain executable without persisting toolbar layout state. A fully fitting
    composition shall keep its rightmost visible Item, including its label, inside the actual
    clipped toolbar content right edge after panel, frame, and viewport insets are applied. Items
    with an approved typed width override, including the Elements control, shall use the same
    resolved width for allocation and rendering without changing the shared padding, gap,
    flexible-space, overflow, or clipped-edge budget.
  - **AC-WRITING-005.6:** On macOS, the desktop shall use the OS-owned native window header and
    place the toolbar at the top of the window content area below it. Header interactions shall
    remain OS-owned. This criterion does not prescribe a particular OS double-click outcome or
    promise a native visual result.
  - **AC-WRITING-005.7:** Toolbar action controls and the Elements control/label surface shall
    request the standard arrow cursor, while the whole-document editing response shall request
    the text/I-beam cursor. Decorative labels and painted page rectangles shall not create
    competing cursor owners.
  - **AC-WRITING-005.8:** The toolbar panel's exact height shall derive from its top padding, shared
    Item-group height, and any explicitly owned trailing padding. Its outer composition shall not
    add implicit vertical gaps, and child content shall remain within its allocated Item-group
    height rather than expanding the returned panel boundary. Its lower divider shall align with the
    panel edge, and Script View shall follow the toolbar without an intervening application spacer
    or fixed frame/top-content inset. Script View page-edge spacing remains owned by scrollable
    content. The toolbar divider shall use an explicit toolbar-local width setting and the toolbar
    palette's border color, without adding other toolbar frame edges.

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
    behavior. Sprint Timer, Focus Mode, Appearance, the page-zoom slider, and the current-percentage
    control shall remain permanently inline at supported widths, in that order, with no status
    overflow disclosure or alternate action representation. No other status action or icon
    assignment shall be introduced without authorization. Sprint Timer shall show only the outlined
    Timer symbol, Focus Mode only the outlined Location Searching symbol, and Appearance only the
    filled Mode Night symbol; these controls shall have no visible captions, and Appearance shall
    have no closed-control arrow. These native Material glyphs shall use the app's registered
    Material Symbols Rounded faces with shared defaults weight 200, optical size 24, and grade 0;
    outlined and filled identity shall be selected by the `FILL` axis. Toolbar icons may override
    weight, optical size, and grade independently per icon, with omitted axes inheriting the named
    defaults. Supported weight choices shall be 100–700 in 100-unit increments; supported optical
    sizes shall be 20, 24, and 48; supported grades shall be -50, 0, and 200. Distinct axis
    combinations shall use separate registered faces so glyph cache results
    cannot cross-contaminate weights. Status icons shall use the shared defaults. Axis choices shall
    not alter icon identity, point size, color, interaction, layout, or document typography. The
    slider shall have no visible caption, and the closed percentage
    control shall show only the current integer percentage without an arrow. Accessible names and
    tooltips shall remain meaningful without visible captions. Focus Mode shall retain its
    state-specific accessible description and shall only be enabled when its existing window-state
    gate permits activation. Status icons and the current-percentage control shall share toolbar hover
    semantics: clipped hovered-widget background, rounded corners, default cursor, and the
    status-local hover padding setting. Percentage hover shall be immediate, shall not activate or
    open its menu, and shall not add timed animation. Idle control backgrounds shall remain
    transparent. Status hover geometry shall not share or change the toolbar's numeric padding.
    Status-owned default/idle text and icon foreground for Timer, Focus Mode, Appearance, percentage,
    and page, plus Diagnostics when enabled through its explicit development gate, shall use one
    status-local theme-aware setting. Center operation/error text shall have its own optional
    status-local foreground override and fall back to that default foreground when unset;
    adjusting the center override shall not recolor other status content. Center text shall also use
    its own status-local logical-point font-size setting, independent of Body, Button, page, and
    Diagnostics typography;
    interactive status hover foreground shall use a separate shared setting. Both settings shall
    expose one documented manual override point while following the active Appearance when the
    override is unset. Popup-menu entries retain menu-owned styling. Pointer and keyboard activation
    shall continue to invoke the existing status actions and accessible behavior.
  - **AC-WRITING-006.2:** The status bar shall report the current operation status without
    displaying a cumulative frame count, project path, or separate Clean/Dirty indicator. Clean
    and dirty state remain owned by the live editing session and shall not be changed by status
    presentation. A clean in-memory project without a named path shall not be described as
    previously saved.
  - **AC-WRITING-006.3:** Operation errors shall remain visibly identifiable, and bounded
    diagnostics shall remain reachable without exposing screenplay text, source text, serialized
    project contents, or unnecessary filesystem paths. Save errors shall retain a concise
    privacy-safe category distinguishing session preparation, codec serialization/re-read,
    storage, and missing-baseline failures. A Save/Save As codec `UnsupportedConstruct` diagnostic
    shall retain its stable integer call-site prefix at character zero of the underlying diagnostic
    text and through transactional and desktop error details; the integer shall map to the exact
    function and return/propagation site in the centralized technical-context map. Other error
    categories shall not acquire a writer code. A normalized source-tree code 30 diagnostic shall
    preserve its integer prefix and append one deterministic, bounded child-index path plus a
    structural field/category through codec, transactional, and desktop error details. It may report
    safe local names, categories, counts, or value-free text length/whitespace classification, but
    shall not expose screenplay or XML node text, attribute/namespace values, IDs/UUIDs, anchor or
    semantic identity values, serialized XML, image data, or filesystem paths. The strict comparison
    and refusal remain unchanged. Save-failure wording shall state that the document is unchanged.
    Status Diagnostics presentation is disabled by default through one manual source constant and may
    be enabled only as an explicit development choice requiring rebuild and relaunch. Disabled mode
    gates the detailed status projection and disclosure only; privacy-safe operation errors and the
    operational counters/instrumentation remain. No profile condition, Cargo feature, runtime
    preference, or document-persisted setting controls this gate.
  - **AC-WRITING-006.4:** Allocate the status row by priority: reserve Sprint Timer, Focus Mode,
    Appearance, the page-zoom slider, and the current-percentage control inline at the far right;
    retain the visible-page indicator in a separately left-aligned, bounded allocation; and allocate
    the operation/error message in a region centered on the status-row content, not between unequal
    siblings. For row content width `W` and the actual full right-group width `R` (from the row's
    right edge to Timer's left edge), the nominal center region shall have width `max(W - 2R, 0)`,
    left edge `row_left + R`, and right edge `row_right - R`. When `W <= 2R`, collapse it to zero
    width at Timer's left edge. The page allocation shall be capped before the mirrored center
    region and right controls so it cannot overlap either; its symmetric horizontal inset shall be
    a named status-local setting applied consistently to measured and
    allocated width, paint clipping, and accessibility bounds. Grow the slider within its named
    status-local width bounds only after fixed right controls and the bounded page allocation fit.
    The center message shall be horizontally and vertically centered
    in its explicit region, clip or disappear when that region is exhausted, and never displace or
    reclaim space from the right controls. There shall be no status More/! More trigger, overflow
    path, or breakpoint-based compact mode. At supported widths including 1200, 480, and 320
    logical points, the five right-hand elements shall remain inline and within the visible row;
    page text clips within its own region if necessary. Operation/error text shall clip horizontally
    as single-line content rather than wrap or grow the footer, and the existing accessible error
    projection shall remain available. One named status-local setting shall own the ordinary row
    height, and the bottom panel's default height shall derive from that row and the existing
    symmetric vertical insets, without an independent total-height setting. The caller shall use
    default rather than exact panel sizing. The ordinary row shall retain its current effective
    height; explicitly enabled Diagnostics may expand through normal panel allocation, while the
    production-off mode adds no row. The symmetric top/bottom panel inset, zoom slider bounds,
    center font size/color, and page inset shall be named, status-local manual settings initialized
    to their current effective values; adjusting center settings shall not alter Body, Button, page,
    or Diagnostics typography/colors. The privacy-safe Diagnostics disclosure and detailed status
    projection shall be present only when the manual development gate is enabled; disabled mode shall
    allocate no Diagnostics row, body, placeholder height, focus target, or accessibility node. Below the minimum physically
    usable viewport, including zero width, layout and hit regions shall remain bounded and
    nonoverlapping, but usable hit targets are not promised. Status panel horizontal and vertical
    insets, icon slot width, inter-item gap, percentage side reservation, zoom slider minimum and
    maximum widths, page-counter horizontal inset, center font size, and status-local text/icon sizes
    shall be owned by named status-presentation settings initialized to their current effective
    values; their
    adjustments shall not mutate toolbar or global framework styling. Measure one stable percentage
    slot from the maximum supported label and active status Button font, plus the existing leading and
    trailing reservations; digit-count changes shall not move the other right controls or change the
    residual center region. Center the current label in that slot. The measured post-slot trace showed
    the planned and actual percentage rectangles sharing the same right edge; preserve that explicit
    rectangle authority rather than compensating through panel or percentage insets. The stock slider
    response shall fill its planner allocation, avoiding an unintended gap before the separately
    anchored percentage slot.
  - **AC-WRITING-006.5:** Given the actual rendered sheet rectangles and actual editor scroll clip,
    the visible-page Item shall use one-based numbering and select the sheet with the greatest
    vertical overlap; exact overlap ties shall select the earlier rendered sheet. If no sheet
    overlaps, it shall select the nearest vertical interval, with exact-distance ties selecting
    the earlier sheet. The total shall include every rendered sheet, including an optional title
    page.
  - **AC-WRITING-006.6:** Visible-page state shall be independent of caret/source position and
    remain current after scrolling, resize, presentation-scale changes, layout reflow, and
    conditional caret reveal. For the active document, retain the most recent page and total
    selected from accepted fresh geometry while same-document geometry is invalid, missing, stale,
    explicitly invalidated, or undergoing refresh/failure; replace that value only after another
    accepted fresh geometry selection. Startup and immediately after successful New or Open shall
    show unavailable until the replacement/current document publishes valid geometry. Failed Open
    and same-document Save shall not clear the retained value. Session revisions are not document
    identity, so a successful replacement shall clear both the published snapshot and retained
    value even when revisions alias. Page-status updates shall not use caret/source position as a
    fallback, scroll the editor, reveal the caret, steal focus, persist view state, or change editor
    input ownership.
  - **AC-WRITING-006.7:** The desktop status bar shall provide one accessible, apparently continuous
    whole-percent page zoom slider from 75% through 300%, with integer stepping, proportional
    trailing fill, hidden value text, and no visible caption. Its accessible name shall identify
    Page zoom and expose the current value and inclusive minimum/maximum. An adjacent arrowless
    current-percentage button shall display every intermediate integer accurately and open a menu
    containing exactly 75%, 90%, 100%, 125%, 150%, 175%, 200%, 250%, and 300%, in that order. Both
    controls shall share one whole-percent page-zoom value. Measure a stable percentage allocation
    from the maximum supported label (300%) in the active status Button font plus the existing leading
    and trailing reservations. Keep the actual current whole-percent label for painting and the
    accessible current-value name, centered horizontally and vertically in that fixed slot. The
    planner's percentage rectangle shall be authoritative for allocation, interaction, clipping,
    accessibility bounds, popup anchoring, and painting; do not reconstruct a second position from a
    nested sequential layout. If narrow-width compression makes the current galley not fully fit,
    keep its origin bounded and clip within the allocation without expanding or overlapping controls.
    The slider shall use a full circular
    knob colored from the same status-local default/hovered foreground settings as status text and
    icons, without recoloring its rail or trailing fill. Its rail thickness shall use a status-local
    setting; no optional reduction is required. Preserve planner-owned slider allocation, stock
    slider response, input and accessibility behavior, and page-zoom semantics. Size the stock slider
    through its scoped egui slider-width setting from the planner's allocation without a separate
    sizing constant. Paint the custom knob after the ordinary
    Timer, Focus Mode, Appearance, slider, and percentage controls, using the
    stock slider response and current zoom value. Clip it to the intersection of the overall
    right-hand group, status row, and inherited parent clip, not the slider response rectangle; its
    normal stroke and interaction expansion may use the right-group gaps but shall not escape the row
    or parent clip. Keep it on the ordinary status paint layer so popup menus and unrelated overlays
    retain their normal ordering. At 100%, page
    presentation width shall use the effective paper width in document points multiplied by the
    platform's logical-points-per-document-point conversion: 1.0 on macOS, 96/72 on Windows, and
    neutral 1.0 on unsupported platforms. The selected percentage shall be retained across resizes.
    The page shall be centered within the
    actual editor viewport when it and its 16-point exterior side buffers fit, and remain
    reachable by horizontal as well as vertical navigation when they do not. Zoom shall scale
    only page presentation without changing editor source mapping or focus/IME ownership. Its
    whole-percent value is disposable native view state in direct-root FDX TextState: Open restores
    it once, and transactional Save/Save As overwrite it from current zoom. Zoom changes alone shall
    not alter semantic Project content, session revision, dirty or undo state, fixed
    pagination/source ranges, or fixed-layout PDF output, and shall not globally scale toolbar/status
    controls. OS conversion applies only to desktop presentation; shared document points, fixed
    pagination, source mapping, and PDF geometry remain OS-independent. Persisted TextState Scaling
    is only the user's whole-percent zoom and never stores platform or native display multipliers.
  - **AC-WRITING-006.8:** With status content and viewport size unchanged, the bottom status panel
    shall remain height-stable across consecutive egui frames at normal and compact widths, with no
    monotonic growth after initial layout;
    status controls and content shall remain available and the panel shall leave both toolbar
    and editor visible. Center message sizing shall remain a stable one-row presentation; changing
    only its center-local font or color override shall not modify Body, Button, page, or Diagnostics
    styles or add status rows.
  - **AC-WRITING-006.9:** The rendered status content and, when the explicit development gate is
    enabled, expanded Diagnostics summary shall remain single-line and clipped to their allocated
    regions at wide, breakpoint-adjacent, compact, and narrow widths. Center operation/error text
    shall be centered within its explicit allocated region while retaining the full accessible text;
    the visible-page counter shall remain left-aligned and clipped to its own inset allocation.
    Changes in long status text or diagnostic counters shall not add rows or increase stable footer height; the enabled Diagnostics
    disclosure retains its collapsed default and opening it may add only its fixed disclosure row.
    With the gate disabled, no Diagnostics row, body, placeholder height, focus target, or accessibility
    node shall be allocated.
  - **AC-WRITING-006.10:** A supported native inward/outward pinch event shall adjust the same
    transient per-window whole-percent page zoom as the status slider and presets, preserving
    the inclusive 75–300% limits and all document, session, pagination, export, and editor-input
    invariants in AC-WRITING-006.7. Inward pinch is the negative direction of the backend's
    supported zoom event; delivery is platform/backend dependent and no distinct squeeze event
    is promised. A valid finite positive OS-native pixels-per-point value, including 1.0, shall
    be applied once with UI zoom 1.0; when the native value is absent or invalid, the effective
    UI scale shall use neutral 1.0. A later valid native value shall replace the fallback.
    UI scale is independent of page zoom and shall not multiply native scale twice.
  - **AC-WRITING-006.11:** Appearance shall offer exactly System, Day Mode, Night Mode, and Midnight
    Mode through an accessible control and begin each app window in transient System mode.
    System shall follow the theme resolved by the desktop framework, including changes reported
    while selected; when no system preference is reported, the framework's resolved fallback
    shall be used without claiming an observed OS theme. Day shall use a light page and
    Night/Midnight dark pages; Midnight's page and major app surfaces shall be measurably darker
    than Night. Page text and IME text shall retain at least 4.5:1 contrast against their
    composited backgrounds, selected text at least 4.5:1 against the selection, and the caret
    and visible selection boundary at least 3:1 against adjacent page colors. The visible page
    selection boundary shall use the centralized selection-boundary token independently from its
    fill and selected-text colors. Appearance changes
    shall not persist and shall not change screenplay data, layout/source mappings, session
    revision, dirty/undo state, page zoom, or PDF/plain-text output. Desktop colors shall be owned by
     one application-level semantic palette, with distinct toolbar, Script Workspace, status, shared
     control, overlay, and feedback designations. The palette shall map into the framework's native
     visuals while retaining app-specific page, status, and toolbar colors; per-item/icon overrides
     remain explicit higher-precedence exceptions. Palette ownership shall not move into screenplay
     data or shared document layout. Toolbar and screenplay-page borders shall use separate named
     local width settings and their respective palette colors; the page border shall retain its
     middle-stroke kind and page-painter clipping. The status frame shall remain stroke-free and the
     bottom panel's framework separator shall be disabled. One explicit full-width top divider shall
     use the status palette border color and a named status-local width, painted within and anchored
     to the outer status-panel rectangle; the status frame shall not add side or bottom borders.
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
- Document and PDF geometry use OS-independent document points (72 points per inch). Only the
    desktop page presentation converts those points to platform logical coordinates and then applies
    the native display multiplier once; absent or invalid native scale is neutral 1.0.
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

# Feature-Area Blueprint: Screenplay Editing

> This feature composes semantic editing, shared commands, and desktop input so writers can revise canonical screenplay content safely across supported interactions.

## Scope and requirements

- **Requirements:** [Screenplay editing](../../requirements/features/screenplay-editing.md); [Desktop workspace](../../requirements/features/desktop-workspace.md) owns `REQ-WRITING-LOOP-011`, whose clipboard criterion is `AC-WRITING-011.3`; [Native document lifecycle](../../requirements/features/native-document-lifecycle.md) owns the shared `REQ-WRITING-LOOP-007` statement.
- **Technical context:** [Technical context](../technical-context.md)
- **Requirements traced:** `REQ-WRITING-LOOP-001`, `REQ-WRITING-LOOP-007`, `REQ-WRITING-LOOP-011`, `REQ-WRITING-LOOP-012`; `AC-WRITING-011.3`.
- **Area summary:** Keyboard, pointer, menus, toolbar, and clipboard actions converge on one typed operation path over the current editing session. Exact open element-type keys and ordered document-owned definitions are semantic authority; canonical text and formatting are edited independently of display, selection, and host presentation.

## Shared composition

- [ScriptCraft Desktop](../containers/scriptcraft-desktop.md) owns focused input collection and application coordination.
- [Document Model](../components/document-model.md) owns canonical Project/Body values, exact open element-type keys, ordered document-owned definitions, text runs, and formatting in [`project.rs`](../../../crates/scriptcraft-model/src/project.rs), [`element.rs`](../../../crates/scriptcraft-model/src/element.rs), and [`formatting.rs`](../../../crates/scriptcraft-model/src/formatting.rs); [`position.rs`](../../../crates/scriptcraft-model/src/position.rs) owns position/selection value primitives. [Command contract](../components/command-contract.md) owns typed activations; [Editing session](../components/editing-session.md) owns edits, history, and live selection over those model values; [FDX preservation and storage](../components/fdx-preservation-and-storage.md) separates model-owned retained source from FDX codec and session storage; [Layout and pagination](../components/layout-and-pagination.md) supplies position mapping; [Desktop adapters](../components/desktop-adapters.md) projects interaction and clipboard effects.

## Capability realization

### Open-type writing and reversible edits

- **Requirements:** `REQ-WRITING-LOOP-001`, `REQ-WRITING-LOOP-007`; `AC-WRITING-001.1`–`AC-WRITING-001.5`, `AC-WRITING-007.1`–`AC-WRITING-007.2`.
- **Composition and flow:** The session exposes an ordered model whose elements use exact open type keys and resolve through the Project's ordered definitions. Input adapters translate keyboard/IME events into text, paragraph, and type commands; menus and Elements controls create the same typed operation. Open command payloads carry exact element-type keys and validate membership against the current Project registry; version 2 remains open, while legacy role spellings are confined to the private version-1 adapter. The headless executor commits canonical mutation before deterministic owner-local source projection; only current semantic and projection invariants gate publication. Imported XML topology, identities, anchors, attachments, references, and run partitions do not veto a semantically legal edit. Definition font/style values affect display only. Enter/Tab guidance is a read-only projection of the same current-definition Behavior resolver rather than a second policy implementation. Arbitrary labels remain distinct; exact base-label capabilities provide only the corresponding structural semantics, and required-base completion does not close the editable vocabulary.
- **Boundary:** Desktop rendering and resolved typography consume the canonical-to-display/source map; they do not normalize text or persist transient policy. Genuine semantic limits, current-model construction defects, and operational source-container/identity-allocation failures remain transactional; imported source representation is not a general edit safe-stop. Behavior execution, document-driven controls, and source reconciliation use the same current definitions and retained-source projection. Closed role spellings survive only in the private version-1 command-wire adapter; they do not define document semantics. The removed legacy JSON codec/state/schema has no persistence or recovery path.

### Phase 4 dynamic editing contract

Return/Tab execution and read-only guidance share current-document exact-key Behavior resolution. Absent or unresolved targets are consumed nonmutating no-ops; no static transition or Action/General fallback participates. Exact-label capabilities alone govern strict Scene Heading/Character/Transition boundaries, Dialogue interior Enter, blank-Character Tab, Parenthetical editing, scene/cast semantics, and dual recognition. Arbitrary and case-variant labels remain editable but receive no guessed semantics; `PaginateAs` is layout-only. Ordinary split retains its exact key, join requires exact key equality, and generic Type-only changes preserve all non-Type content and source ownership. Automatic Parenthetical generation belongs to high-level editing transitions/assignment, not generic Type replacement/remapping. These editing decisions and transaction outcomes are headlessly observable; physical desktop shortcuts, controls, focus/IME routing, and rendered guidance wait for the grouped Phase 4–6 desktop gate.

### Selection, navigation, and clipboard

- **Requirements:** `REQ-WRITING-LOOP-012`; `AC-WRITING-012.1`–`AC-WRITING-012.8`, `AC-WRITING-011.3`.
- **Composition and flow:** Script View converts hit-test geometry into semantic document positions and sends selection/navigation commands through the headless executor. Keyboard shortcuts are accepted only by the focused editor adapter under the supported modifier/input-method gates. Copy serializes selected canonical body text; Cut writes that text to the host clipboard before dispatching deletion; Paste reads only on activation and dispatches nonempty text only after a successful read. All committed edits and selection changes share session validity and history rules.
- **Boundary:** Clipboard objects, pointer coordinates, focus, IME preedit, and physical key details remain in the desktop adapter. Only typed text, semantic positions, directions, and selection endpoints cross the command boundary.

### Editor invisibles overlay

- **Requirements:** `AC-WRITING-001.6`; [Native Document Lifecycle](../../requirements/features/native-document-lifecycle.md) owns FDX view-state import and transactional output; [Exports and Print](exports-and-print.md) owns output exclusion.
- **Composition and flow:** Desktop Script View paints a small dot at each projected U+0020 boundary and a pilcrow at paragraph endings and imported CR/LF line endings; an imported CRLF pair receives one line-break marker. Marks use canonical presentation positions, the document's resolved invisibles color, zoom-scaled sizing, and the existing per-line text clip. The independent Body flag gates all Body modes; the independent Title Page flag gates its editor-purpose Page presentation through the shared painter.
- **Boundary:** Marks are paint-only, noninteractive, and absent from Print-purpose layouts. They do not change canonical text/advances, wrapping, pagination, layout/cache identity, selection/caret/pointer/IME geometry, clipboard, undo/redo, or dirty state, and no PDF, plain-text, or native-print path consumes the overlay.

## Decisions

- Route every enabled editing action through the shared command and session path; do not let widgets mutate project state directly.
- Keep canonical text, display transformations, and transient input state separate.
- Superseded contract: the fixed Role table and static transition policy are replaced by open exact type keys, ordered definitions, and current Behavior/capability resolution. The private version-1 command-wire adapter is not a model Role authority or a document-persistence compatibility layer.

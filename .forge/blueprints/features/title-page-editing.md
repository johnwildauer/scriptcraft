# Feature-Area Blueprint: Title Page Editing

> This feature composes a distinct title-page model and native view with the existing document session, command path, shared history, layout, and FDX persistence.

## Scope and requirements

- **Requirements:** [Title page editing](../../requirements/features/title-page-editing.md); [Desktop workspace](../../requirements/features/desktop-workspace.md) owns `REQ-WRITING-LOOP-005` for `AC-WRITING-005.10`; [Native document lifecycle](../../requirements/features/native-document-lifecycle.md) owns the shared `REQ-WRITING-LOOP-007` statement.
- **Technical context:** [Technical context](../technical-context.md)
- **Requirements traced:** `REQ-WRITING-LOOP-005`, `REQ-WRITING-LOOP-007`; `AC-WRITING-007.3`, `AC-WRITING-007.5`–`AC-WRITING-007.9`, `AC-WRITING-005.10`.
- **Area summary:** Title Content is a separately editable surface of the same native project, not another element type, project, or process. Only title-page Content paragraphs are managed and editable; their optional exact type keys use the same open ordered registry as Body. Type-less Content paragraphs remain valid, while other title structure remains opaque and source-preserved. Surface-specific controls feed the same session and history.

## Shared composition

- [Document Model](../components/document-model.md) owns the canonical title paragraph values and formatting in [`title_page.rs`](../../../crates/scriptcraft-model/src/title_page.rs), composed in [`project.rs`](../../../crates/scriptcraft-model/src/project.rs). [Editing session](../components/editing-session.md) owns edits, title selection/typing state, and shared undo/redo over those values.
- [Command contract](../components/command-contract.md) provides surface-aware edit commands; [FDX preservation and storage](../components/fdx-preservation-and-storage.md) separates model-owned retained title source and FDX title codec from session-owned transactional native save; [Layout and pagination](../components/layout-and-pagination.md) provides title-page composition; [Desktop adapters](../components/desktop-adapters.md) owns the singleton native title window and its toolbar/status presentation.

## Capability realization

### Separate editable surface in one session

- **Requirements:** `AC-WRITING-007.3`, `AC-WRITING-007.5`–`AC-WRITING-007.7`.
- **Composition and flow:** The application session retains the ordered editable title Content occurrences independently from Body paragraphs. Each Content paragraph may have an exact open type key from the Project definition registry or no type key. The title surface does not infer a type from occurrence position and does not coerce type-less Content into a definition. Non-Content title structure remains opaque under source preservation. Opening the Title Page command creates or focuses the application-managed title window; it binds the existing project/session and title surface. The title view requests title-specific layout, selection, and transient zoom, while appearance and timer remain shared per-window application state. A focused collapsed Title Page caret resolves and paints through the same shared Body geometry and renderer, including focus, blink, and clipping, without expanding projection beyond the existing caret page; range selections and unfocused surfaces show no caret. Its status surface contains page count, Timer, Appearance, zoom slider, and percentage, with no Body center guidance or hidden Focus Mode slot, gap, input, or accessibility node; Timer and Appearance use ordinary adjacent-item spacing. Closing the title view affects only that viewport; root document closure coordinates its teardown.
- **Boundary:** The main Script View remains body-only. Opaque title structure is not converted into editable Content, and transient title-view state does not initialize or persist as document content.

### Formatting, history, and native save

- **Requirements:** `AC-WRITING-007.8`–`AC-WRITING-007.9`, `AC-WRITING-005.10`.
- **Composition and flow:** Formatting controls create explicit Title Page surface style/alignment commands. The command executor delegates to the same session operations and history used by Body; semantic selection projection determines control state. Title layout maps display-only styles back to canonical source positions. During Apply Template, exact typed Content keys are retained, unmatched typed Content keys remap to target Action, and type-less Content plus opaque title structure remain unchanged. Native Save/Save As passes editable title Content through the bounded source projection and candidate publication path, then rebases the same session while preserving both surface selections. New from Template retains the selected template's complete title Content sequence and uses presentation title `Untitled ` plus the selected built-in catalog display name or external FDXT source filename stem with only the final extension removed (default Screenplay remains `Untitled Screenplay`); that presentation identity does not alter title content, history, or persistence.
- **Boundary:** Title widgets never persist independently or fabricate screenplay element types. Source-owned headers, footers, non-Content title structure, opaque paragraphs, and repeated foreign IDs remain under the preservation component rather than title presentation.

## Decisions

- Use one semantic document session and history with explicit surface identity, not a second document/editor model.
- Keep the native title window as a desktop adapter concern; title values belong to the model, edits/history to session, title composition to layout, and FDX encoding/source projection to the FDX/session boundary.
- Superseded contract: title paragraphs are not a closed role inventory. Only Content is editable; its type is optional and, when present, resolves through the current open definition registry.

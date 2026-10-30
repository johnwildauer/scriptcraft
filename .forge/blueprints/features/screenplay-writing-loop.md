# Feature-Area Blueprint: Dependable Screenplay Writing Loop

## Scope and requirements

- **Requirements:** [`../../requirements/features/screenplay-writing-loop.md`](../../requirements/features/screenplay-writing-loop.md)
- **Technical context:** [`../technical-context.md`](../technical-context.md)
- **Area summary:** A local desktop writing surface delegates semantic screenplay operations to a reusable headless core. The core project remains the single canonical source; transactional editing, shared fixed-profile layout, native persistence, and derived exports preserve screenplay meaning from input through review output.

## Shared composition

The [ScriptCraft Desktop container](../containers/scriptcraft-desktop.md) owns the window, keyboard/IME and pointer input, editor and page presentation, file selection, clipboard, toolbar, status, and platform interactions. It adapts user actions to the [Screenplay Core component](../components/screenplay-core.md), which owns the semantic project and editing session, role-transition policy, fixed-profile layout and source mappings, native serialization, and export adapters. The desktop consumes core results and handles presentation and filesystem/platform operations; the core is a reusable library, not a separately runnable service. There is one authoritative semantic document, not a second desktop text buffer.

The flow is: writer input → desktop adapter → transactional core session/project → shared fixed-profile layout → desktop presentation or PDF. Save/open crosses the desktop file boundary through core native-project serialization; plain text is a separate lossy export. Window and presentation state stays outside canonical project data. Component and container blueprints define their detailed contracts; this feature composes them without duplicating their inventories.

## Capability realization

### Semantic screenplay editing

- **Requirements:** `REQ-WRITING-LOOP-001`; `AC-WRITING-001.1`–`.5`, including `.4a` and `.4b`.
- **Composition and flow:** The core project stores ordered screenplay elements with stable opaque IDs, one of seven explicit roles (Scene Heading, Action, Character, Dialogue, Parenthetical, Transition, Note), canonical UTF-8 text, metadata, title-page content, and fixed page-profile inputs. A new project has one empty Scene Heading and a valid caret at element 0, byte 0. The desktop sends editing and navigation commands to the core session; it does not infer roles from appearance or own separate transition policy.
- **Contracts:** Checked UTF-8-safe, bounded mutations are transactional, undoable where applicable, and preserve project/session state on rejection or failure. Enter and Tab use explicit role policies, including the collapsed Dialogue-end transition to Action and the interior Dialogue split: the original ID and prefix remain, a distinct bounded ID owns the Dialogue suffix, and the caret moves to its start. Cue casing applies only at edit boundaries to Scene Heading and Character; opening does not rewrite stored text, and absent legacy casing settings use the specified default.
- **Parentheticals:** Automatic generation creates transient `()` with an interior caret and stable-element provenance that participates in undo/redo but is not serialized. The special Enter cleanup applies only to a still-marked generated empty pair. Pasted parentheses remain intact during paste and thereafter follow ordinary document Tab/Delete handling, including delimiter-only transitions/deletion and the applicable previous-element or first/only-element fallback. This is distinct from generated-pair Enter behavior; authored nonempty content is preserved and rejected operations do not mutate state.

### Shared deterministic writing and page views

- **Requirements:** `REQ-WRITING-LOOP-002`; `AC-WRITING-002.1`–`.4`.
- **Composition and flow:** The core layout derives immutable page, line, and source-range geometry from the semantic project and complete, versioned fixed screenplay profile. The desktop uses the shared geometry for continuous writing, fixed-page presentation, hit testing, caret placement, selection, and visual-line navigation; PDF uses the same fixed print-layout contract.
- **Contracts:** Identical project/profile inputs yield deterministic pagination and UTF-8-byte source mappings independent of viewport zoom. Role-aware wrapping and keep relationships do not insert formatting into canonical text; title-page boundaries remain separate from body spacing, incoming Dialogue and Parenthetical have no additional body boundary gap, and other incoming body roles use the profile gap. Oversized content has a bounded safe layout path. Caret/hit-test mapping stays at valid text boundaries. Cached geometry belongs to the current project and complete profile; revision/profile equality alone must not allow geometry from a replaced project or session to be reused.

### Durable native project persistence

- **Requirements:** `REQ-WRITING-LOOP-003`; `AC-WRITING-003.1`–`.3`.
- **Composition and flow:** The core serializes and parses the versioned canonical project; the desktop owns file selection and routes open/save operations. Only semantic project content and reproducible layout inputs are required in the native representation, not caret, selection, derived layout, undo history, or transient generated-parenthetical provenance.
- **Contracts:** Schema version 1 is supported. Specified missing legacy fields receive their defaults; malformed or unsupported versions are rejected safely, without a cross-version migration commitment. Save prepares bounded data and atomically replaces the named destination. Open/save failures preserve the valid session and prior native destination; associated path and clean state change only after the corresponding operation succeeds. New must not replace a dirty session without the writer's explicit informed consent. There is no snapshot-recovery promise or implied substitute.

### Explicit noncanonical exports

- **Requirements:** `REQ-WRITING-LOOP-004`; `AC-WRITING-004.1`–`.3`.
- **Composition and flow:** Core export adapters consume the semantic project and fixed print layout. PDF and plain text are derived review outputs; desktop adapters handle any applicable file interaction without changing the native-save contract.
- **Contracts:** PDF uses fixed print geometry and preserves non-ASCII characters without prescribing a conflicting font-encoding or embedding mechanism. Plain text is deliberately lossy and communicates that roles, stable IDs, and page layout are not preserved. Neither export replaces the native source or marks it clean, and export failure leaves the native project and in-memory session intact. No particular desktop text-export route is part of this composition.

### Desktop toolbar and native header

- **Requirements:** `REQ-WRITING-LOOP-005`; `AC-WRITING-005.1`–`.7`.
- **Composition and flow:** The desktop toolbar routes project commands and role selection through desktop adapters to the core session, persistence, or fixed-layout export boundary. Elements reflects the selected explicit role and applies a chosen role through the transactional session. On macOS, the operating system owns the native window header; toolbar content begins below it in the application content area.
- **Contracts:** The default action order, icon-first controls, accessible labels, role choices, narrow-width overflow, measured label-aware widths, flexible-space allocation, and clipped-edge behavior follow the linked criteria. Action and Elements controls request the standard arrow cursor; the whole-document editor requests the text/I-beam cursor, with no competing decorative cursor owner. 

### Status, visible page, and transient presentation

- **Requirements:** `REQ-WRITING-LOOP-006`; `AC-WRITING-006.1`–`.13`.
- **Composition and flow:** The desktop projects session cleanliness, operation/errors, privacy-safe diagnostics, and fresh rendered-page geometry into an accessible status surface. Status observes the editor and does not take ownership of semantic input. Per-window zoom, appearance, timer, focus-mode requests, and overflow are transient presentation state, not project, session-history, fixed-layout, or export data.
- **Contracts:** Status distinguishes clean/unsaved state without exposing the project path and selects the visible page from actual sheet rectangles and the editor scroll clip, independently of caret position. Invalid or stale geometry produces an unavailable state until refreshed. Bounded overflow preserves access to page/session/error information and the ordered Timer, Focus Mode, Appearance, and zoom controls; text remains clipped and the footer bounded, with no control guarantee for an unusable viewport. Zoom and appearance do not alter semantic content, fixed pagination/source ranges, native state, or exports. Timer completion remains available in-app while OS notification is best-effort; fullscreen entry/exit is a request reconciled against reported window state, not a guaranteed native transition. Accessibility, transient control details, limits, and conditional behavior remain specified by the linked acceptance criteria and desktop blueprint.

## Constraints

- The native semantic project is the single source of truth; layout, selection, undo, and presentation are derived or transient.
- Editing, file operations, and exports preserve bounded failure behavior and the last valid native project. Fixed pagination is independent of viewport presentation.
- Core editing, layout, and persistence remain exercisable without a desktop window; deterministic output follows identical semantic project/profile inputs.
- **Non-functional requirements:** `NFR-WRITING-001` (typed errors and bounded inputs), `NFR-WRITING-002` (headless core verification), and `NFR-WRITING-003` (deterministic native serialization and structural pagination).
- Native macOS behavior and platform-dependent event delivery are bounded to the explicit contracts above; no unlisted OS behavior is implied.

## Decisions

- **Keep the native semantic project canonical and outputs derived.** This preserves stable identity and saved-state meaning while allowing layout, PDF, and lossy text to be regenerated without replacing the source.
- **Keep core policy independent of desktop presentation.** The headless core owns semantic editing, shared layout, persistence, and export contracts; the desktop owns input, presentation, files, and platform interaction.
- **Support schema version 1 without promising cross-version migration.** Specified legacy defaults and safe rejection of malformed or unsupported versions bound the native compatibility contract.
- **Preserve the recorded editing and platform intent.** Pasted parentheses follow normal subsequent Tab/Delete rules, distinct from generated-pair Enter cleanup; PDF preserves non-ASCII; dirty New requires informed consent; and the macOS header is OS-owned rather than an app-owned title strip.

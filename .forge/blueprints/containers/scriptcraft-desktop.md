# Container Blueprint: ScriptCraft Desktop Application

## Container Summary

ScriptCraft Desktop is the desired native runnable boundary for an individual
screenwriter. It owns the window, input/IME path, focused editor, page preview,
dialogs, clipboard, file selection, status/error presentation, and platform
services. It delegates semantic edits, layout decisions, and persistence
contracts to the core rather than maintaining a second authoritative text
buffer.

## Document status

- **Status:** `draft` — desired boundary; not implemented or independently reviewed
- **Owner:** Forge initialization agent
- **Last updated:** 2026-10-25
- **Technical context:** [`../technical-context.md`](../technical-context.md)
- **Evidence basis:** desired architecture from `.analysis`, plus verified WO-009/WO-010 app-boundary behavior

## Infrastructure

- **Technology/runtime:** Rust native desktop process — language confirmed; framework and OS target unknown.
- **Build/package:** Cargo is an agent-default pending manifest verification; exact command unknown.
- **Deployment/distribution:** Local desktop executable/package; packaging method unknown and out of current initialization scope.
- **Configuration:** Keep UI preferences/session state separate from native screenplay projects; exact preference location unknown.
- **Dependencies:** Core capability and selected desktop UI framework; do not add MCP, web, or broad format dependencies for MVP.

## Entry points and boundaries

- **Inputs:** Keyboard/IME text, semantic role actions, selection/navigation, file dialogs, open/save/export requests, and page-preview interactions.
- **Outputs:** Updated session state, rendered continuous/page views, status/errors, native project writes, PDF output, and lossy text output.
- **External systems:** Local filesystem and desktop windowing/clipboard APIs; no network/authentication/MCP in MVP.
- **Internal areas:** `screenplay-core` capability, UI adapter, platform file service, and renderer/export adapters.

## System Contracts

### Key Contracts

- The desktop shell never becomes the source of truth for screenplay text.
- Every semantic mutation goes through the core editing/session boundary and is undoable when it changes the project.
- Viewport size and zoom do not redefine print pagination.
- A failed open/save/export reports an error and does not silently replace valid in-memory or on-disk project state.

### Integration Contracts

- UI-to-core contract: typed semantic operations/results; exact API unknown until implementation.
- Core-to-render contract: backend-neutral pages/display data with source mappings; exact representation desired but unimplemented.
- Core-to-filesystem contract: versioned native project bytes, atomic replacement, bounded recovery candidates, and explicit export status.

## Architecture Decision Records

### ADR-001: Desktop-first, core-independent boundary

**Context:** The product is desktop-first, but dependable behavior must be testable without a window and future platform work must not force a semantic rewrite.

**Decision:** Keep the screenplay core independent of desktop UI APIs and place window/input/platform services at this container boundary.

**Consequences:** Headless tests and future hosts remain possible; exact framework selection is deferred to workspace discovery.

## Completion evidence placeholder

`TODO: link verified package manifest, build/run evidence, desktop smoke evidence, and independent blueprint review.`

## Revision history

| Date | Change | Evidence/rationale |
|---|---|---|
| 2026-10-25 | Created MVP desktop container boundary | User-approved desktop-first scope and `.analysis`; implementation/framework not claimed |
| 2026-10-25 | Recorded foundation app seam | WO-001 verified a dependency-free native Rust binary and headless startup path; visible UI remains deferred |
| 2026-10-25 | Recorded unified editor and named persistence host behavior | WO-009/WO-010 implementation evidence; app owns the optional current named path and native `rfd` selection, while core owns JSON and atomic-save semantics; recovery remains outside the UI |

## WO-001 verified foundation facts

- `apps/scriptcraft` is the runnable Rust binary boundary and targets the native `aarch64-apple-darwin` host observed by `rustc -vV`.
- WO-001 intentionally selected no UI/window framework dependency; `cargo run -p scriptcraft` prints `scriptcraft foundation ready`.
- The visible-window smoke path is unavailable/deferred in this foundation WO, not claimed as passed.

## WO-006 verified persistence facts

- The app host owns [`StdFileSystem`](../../../crates/scriptcraft-core/src/persistence.rs) and path selection, while the core owns bounded bytes, atomic-save semantics, and recovery candidate contracts.
- The focused host exposes native file-dialog Open, Save, and Save As actions. The app owns the optional current named path and displays it in status; Open replaces the session and path only after successful parse/validation, while Save marks the session clean and updates the path only after atomic replacement. Recovery APIs remain core-only and are not exposed in this UI.

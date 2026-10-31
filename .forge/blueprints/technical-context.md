# Technical Context

This map describes ScriptCraft's desired technical structure and runtime boundaries. It is a navigation aid, not an implementation inventory.

## Project map

| Area | Path | Role in the target structure |
|---|---|---|
| Cargo workspace | [`Cargo.toml`](../../Cargo.toml) | Defines the workspace for the desktop application and reusable core crate. |
| Desktop application | [`apps/scriptcraft/`](../../apps/scriptcraft/), [`main.rs`](../../apps/scriptcraft/src/main.rs), and [`src/`](../../apps/scriptcraft/src/) | Runnable local desktop shell and its UI/platform modules; owns presentation and native file interaction. |
| Screenplay core | [`crates/scriptcraft-core/`](../../crates/scriptcraft-core/), [`lib.rs`](../../crates/scriptcraft-core/src/lib.rs), [`editing.rs`](../../crates/scriptcraft-core/src/editing.rs), [`layout.rs`](../../crates/scriptcraft-core/src/layout.rs), [`persistence.rs`](../../crates/scriptcraft-core/src/persistence.rs), and [`exports.rs`](../../crates/scriptcraft-core/src/exports.rs) | Reusable headless library for semantic editing, shared layout, native-project persistence, and noncanonical export adapters. |
| Core verification | [`crates/scriptcraft-core/tests/`](../../crates/scriptcraft-core/tests/) | Automated tests for core behavior independently of the desktop UI. |
| Bundled typeface | [`fonts/CourierPrime/`](../../fonts/CourierPrime/) | Bundled Courier Prime font assets used by the screenplay experience. |
| Product and technical intent | [`requirements/`](../requirements/), [`blueprints/`](./) | Durable product requirements and architecture descriptions; start at the [Forge index](../_index.md). |

## Runtime and delivery

ScriptCraft runs as a local Rust desktop application built and launched through the Cargo workspace. The desktop application is the independently runnable product boundary. It delegates screenplay semantics and shared document operations to the headless-capable core library, which is reusable by the desktop rather than a second runnable container. No distribution mechanism or deployment pipeline is prescribed here.

## Technology and boundaries

The workspace uses Rust and Cargo. The desktop boundary contains the eframe/egui presentation layer and native-file-dialog integration; UI and platform concerns stay outside the reusable core. The core owns semantic session/project operations, fixed-profile layout, native persistence, and noncanonical export adapters. Framework and dependency versions are implementation choices rather than permanent architecture constraints.

- Desktop shell owns presentation and platform/file interactions. Screenplay Core supplies reusable headless document capability, not an independently runnable container. The [writing-loop requirements](../requirements/features/screenplay-writing-loop.md) describe the composed user-facing flow.

Desktop input delegates semantic operations to the core session and project. The core's shared fixed-profile page layout supplies desktop presentation and PDF generation; viewport zoom affects presentation, not print geometry. FDX Version 6 is the canonical native save boundary and uses atomic replacement. The headless core separates an [FDX codec](../../crates/scriptcraft-core/src/fdx.rs) from filesystem persistence and couples the semantic model to ordered source-preservation state; only managed fields are regenerated. PDF and deliberately lossy plain-text exports are noncanonical outputs. Fountain conversion is a separate secondary adapter, not native persistence. Transient view state remains separate from canonical project content.

## Cross-cutting design

- **Configuration and secrets handling:** This local product architecture does not depend on service credentials or a secrets-management system; no additional configuration or deployment mechanism is specified here.
- **Identity and access:** An account or identity subsystem is not part of the described single-writer desktop product.
- **Errors, reliability, and observability:** Editing operations preserve transactional state. Save failures preserve the native project and leave the session's saved/dirty state accurate. Errors and diagnostics avoid unnecessary exposure of screenplay content.
- **Persistence and data evolution:** Native files are FDX Version 6. Explicit non-6 inputs receive a warning and a Version 6 importer attempt; previous versions use the lossy title/content-only path, while newer handling is best effort without a compatibility promise. Malformed input fails safely. Save validates a complete candidate before atomic replacement and publishes session corrections only after success. Legacy JSON migration is not a product commitment.
- **Verification approach:** Cargo workspace checks and automated headless core tests cover build and core behavior. Desktop interaction and platform integration require applicable desktop/manual checks; automated core checks do not establish native OS behavior.

## Architectural constraints

- Maintain one semantic source of truth in the core; desktop presentation and exports derive from it.
- Keep semantic data bounded and edits transactional. Shared page geometry is repeatable for the same effective runtime layout inputs and independent of viewport scaling; this is not a promise of exact fidelity to every FDX layout. The core remains verifiable without a desktop runtime.
- Keep canonical native-project saving separate from noncanonical PDF and lossy text exports, and preserve failure safety at the native save boundary.
- On macOS, the header is owned by the native OS rather than an app-owned title strip; the application does not prescribe additional behavior for that OS-owned surface.

## FDX model and ownership contracts

- The semantic document retains all 17 directed content types and ordered text/run data. Internal identities and edit anchors are transient implementation state and may be regenerated on reopen; they are not durable ScriptCraft document IDs. The preservation layer owns source order, namespaces, unknown nodes and attributes, foreign/source identities and anchors, FDX-owned metadata, title-page structure, and source document settings, including nested scene data and board/item/image relationships. The codec and editor share a transactionally consistent ownership map; the editor never silently converts Version 6 unknown types to Action. Do not add a private FDX extension or sidecar for ScriptCraft project identity/schema, metadata, layout configuration, or editing policy.
- Layout inputs are runtime-derived from explicit application defaults and only validated FDX mappings. Preserve FDX layout-related source settings even when the renderer cannot interpret them; preservation does not claim those settings are honored or produce exact source-layout fidelity. Nondefault legacy per-project layout configuration is not durable and may reset after reopen. Editing policies such as cue casing, automatic parentheticals, and strict transition boundaries are runtime policy with explicit defaults, not document or FDX fields.
- Import records cast-list text unchanged. A scene-range updater periodically derives managed lists; save checks a detached candidate, corrects silently, serializes it, then publishes that same candidate to the live clean baseline only after successful replacement.
- Script and board objects are independent; their cross-domain links may be silently orphaned and do not block script edits. Preserve board objects and their internal relationships and unrelated authored content where possible. A concrete unsolvable conflict with authored-data preservation is escalated rather than generalized into a ban on script edits.
- Preserve imported title-page structure without position-based metadata guesses; title-page editing is deferred. XML handling is bounded, namespace-aware, denies external entities and external resource resolution, and never dereferences recorded image paths. Unknown mappings and unrepresentable edits stop safely or escalate; preservation is not a claim of lexical identity or interoperability.
- Native Save and Save As serialize fully before replacement, detect conflicts, retain destination and live session on failure, and use platform-appropriate same-directory atomic replacement. Recovery, if retained, carries the preservation state; old JSON recovery is not migration support. Fountain's fixed conversion protocol and implementation remain separate from the native FDX boundary.

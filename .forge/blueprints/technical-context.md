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

- [ScriptCraft Desktop container](containers/scriptcraft-desktop.md) — runnable desktop shell, presentation, and platform/file interactions.
- [Screenplay Core component](components/screenplay-core.md) — reusable, headless semantic and document capability; not an independently runnable container.
- [Screenplay Writing Loop feature](features/screenplay-writing-loop.md) — composition of the user-facing writing flow across desktop and core.

Desktop input delegates semantic operations to the core session and project. The core's shared fixed-profile page layout supplies desktop presentation and PDF generation; viewport zoom affects presentation, not print geometry. Versioned native-project persistence is the canonical save boundary and uses atomic replacement. PDF and deliberately lossy plain-text exports are noncanonical outputs; text export communicates its loss and does not define a separate desktop interaction route. Transient view state remains separate from canonical project content.

## Cross-cutting design

- **Configuration and secrets handling:** This local product architecture does not depend on service credentials or a secrets-management system; no additional configuration or deployment mechanism is specified here.
- **Identity and access:** An account or identity subsystem is not part of the described single-writer desktop product.
- **Errors, reliability, and observability:** Editing operations preserve transactional state. Save failures preserve the native project and leave the session's saved/dirty state accurate. Errors and diagnostics avoid unnecessary exposure of screenplay content.
- **Persistence and data evolution:** Native project files are versioned and are the canonical persisted representation. Save uses atomic replacement; unsupported project versions are rejected safely. Exported files are derived outputs, not replacements for the native project.
- **Verification approach:** Cargo workspace checks and automated headless core tests cover build and core behavior. Desktop interaction and platform integration require applicable desktop/manual checks; automated core checks do not establish native OS behavior.

## Architectural constraints

- Maintain one semantic source of truth in the core; desktop presentation and exports derive from it.
- Keep semantic data bounded and edits transactional. Shared page geometry is deterministic and independent of viewport scaling, and the core remains verifiable without a desktop runtime.
- Keep canonical native-project saving separate from noncanonical PDF and lossy text exports, and preserve failure safety at the native save boundary.
- On macOS, the header is owned by the native OS rather than an app-owned title strip; the application does not prescribe additional behavior for that OS-owned surface.

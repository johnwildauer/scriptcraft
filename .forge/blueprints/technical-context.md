# Technical Context — ScriptCraft

## Document status

- **Status:** `draft` — initialized technical baseline; independent blueprint review not performed
- **Owner:** Forge initialization agent
- **Last updated:** 2026-10-25
- **Evidence basis:** observed repository inventory plus inferred/desired architecture from `.analysis`
- **Completion evidence:** `TODO: link fresh-context blueprint review and verified workspace baseline after the foundation initiative`

## Evidence classification

- **Observed:** `.analysis/`, `.forge/`, `apps/`, and `crates/` exist; no files were returned under `apps/scriptcraft/` or `crates/` during inventory. Git is initialized on `master`, has no commits yet, and has no known remote.
- **Inferred:** The repository is an existing planning/workflow shell whose intended product direction is a Rust desktop screenplay app; the analysis references a broader WordCraft-like Rust codebase, but those referenced files are not current checkout evidence.
- **Desired:** A small dependency-directed Rust core, desktop application shell, headless test path, semantic model, deterministic layout, native persistence/recovery, and explicit export adapters.

## Repository map

| Area | Path/signpost | Responsibility / confidence |
|---|---|---|
| Forge workflow | [`.forge/`](../) | Durable process, requirements, blueprints, and future initiative records — observed |
| Product research | [`.analysis/`](../../.analysis/) | Architecture and MVP research inputs; not executable source — observed |
| Intended desktop app | [`apps/scriptcraft/`](../../apps/scriptcraft/) | Expected desktop host boundary; currently empty in inventory — observed path, desired responsibility |
| Intended reusable code | [`crates/`](../../crates/) | Expected Rust library boundary; currently empty in inventory — observed path, desired responsibility |
| Semantic core (proposed) | `crates/scriptcraft-core/` or equivalent | Project model, roles, positions, editing/session, layout contracts, persistence — desired; exact package path unknown |
| Desktop shell (proposed) | `apps/scriptcraft/` | Window, input/IME, dialogs, clipboard, file host, focused editor — desired; framework unknown |
| Test/smoke boundary (proposed) | Core tests and/or a headless runner | Exercise the same semantic operations as the UI — desired; command unknown |

## Runtime and deployment topology

The desired topology is a native desktop Rust process with a UI shell above a
headless-capable core. The core should not depend on windowing APIs. User input
is translated into semantic editing operations; layout produces backend-neutral
page/display data; the desktop shell renders it and owns platform file/dialog
services. PDF export consumes the same print layout boundary. Exact framework,
entry point, package names, and packaging/distribution are unknown until a
workspace foundation discovers and verifies them.

```text
desktop shell (desired)
        │ semantic user intent
        ▼
session/editing boundary ─────── persistence/recovery boundary
        │
        ├── semantic Project + explicit screenplay Roles
        ▼
fixed PageProfile → layout/pages/source mappings → display list
                                             ├── desktop renderer
                                             └── PDF exporter
```

## Technology baseline

| Concern | Choice | Version/constraint | Confidence |
|---|---|---|---|
| Language | Rust | User-confirmed | observed as user decision, not source evidence |
| Build/package | Cargo | Exact manifest/version unknown | agent-default / unverified |
| Desktop host | Native desktop application | Framework unknown | desired / provisional |
| Core boundary | Window-independent library or modules | Keep semantic behavior headless-capable | desired / inferred from analysis |
| Native data | Versioned UTF-8 JSON-like owned project | Extension/schema not selected | desired / provisional |
| Layout | One fixed named screenplay page profile initially | `us_letter_screenplay`: 612 × 792 units, margins top/bottom/left/right 72/72/90/72, nominal 55 columns × 55 lines; backend-neutral fixed metrics use 7-unit character cells and 12-unit lines | verified by WO-004 implementation; fresh validation pending |
| Export | PDF plus deliberately lossy plain text | Exact library/contract unknown | desired / provisional |

## Cross-cutting concerns

- **Configuration and secrets:** No current application configuration is observed. Desired project files exclude UI/session state; secret policy is not applicable to the initial local MVP and remains unknown for future integrations.
- **Authentication/authorization:** None desired for the single-user local MVP; MCP/remote control is out of scope.
- **Errors and retries:** Desired typed/bounded errors, transactional in-memory edits, failed-save preservation, and explicit recovery candidates. Exact error types are not implemented.
- **Logging and observability:** Unknown. Discovery action: inspect selected framework and establish a minimal user-visible status/error path without treating logs as product source of truth.
- **Persistence and migrations:** Native schema version `1` uses the existing bounded deterministic JSON model; parsing rejects malformed/unsupported versions. The core exposes byte operations and an injected filesystem boundary, while the host supplies filesystem paths. Saves use sibling-temp replacement and recovery retains three revision-named candidates.
- **Testing strategy:** Desired headless model/session tests, hostile-input/rollback tests, structural pagination tests, persistence failure tests, and a small desktop smoke path. Commands are unknown until the workspace exists.

## Boundary inventory

- [`containers/scriptcraft-desktop.md`](containers/scriptcraft-desktop.md) — desired runnable desktop application boundary; exact package and framework require discovery.
- [`components/screenplay-core.md`](components/screenplay-core.md) — desired reusable semantic/session/layout/persistence capability.
- [`features/screenplay-writing-loop.md`](features/screenplay-writing-loop.md) — technical composition of the stable MVP feature.

## Architectural risks and unknowns

- The current source baseline is empty in the inspected application/library paths; do not infer that the analysis-referenced WordCraft modules are present.
- Framework and package structure are unresolved; choose only after a verified workspace discovery step.
- Font metrics can make pagination nondeterministic across machines; pin or bundle an acceptable open font/profile before promising exact output.
- A general document model could reintroduce Word-specific breadth; keep the semantic screenplay model explicit and small.
- Recovery snapshots are not the same as atomic saves; both need separate tests and evidence.
- Incremental pagination, collaboration, interchange, MCP, and web support remain deferred until a demonstrated requirement exists.
- Independent requirements and blueprint reviews have not occurred; this baseline must not be treated as reviewed or validated.

## Revision history

| Date | Change | Evidence/rationale |
|---|---|---|
| 2026-10-25 | Replaced technical-context template with observed/inferred/desired baseline and only necessary MVP boundaries | Workspace inventory plus `.analysis` architecture proposal; no source implementation claimed |
| 2026-10-25 | Recorded WO-001 Cargo workspace and native headless seam | `Cargo.toml`, `apps/scriptcraft`, `crates/scriptcraft-core`, native `aarch64-apple-darwin` target, and passing baseline commands |

## WO-001 verified foundation facts

- The workspace members are `apps/scriptcraft` (`scriptcraft` binary) and `crates/scriptcraft-core` (`scriptcraft-core` library).
- The app depends on the reusable crate; the reusable crate has no UI or window dependency.
- No desktop UI framework was selected in WO-001; the smallest dependency-free native Rust/headless seam is implemented.
- `rustc -vV` reported Rust 1.99.0 on native target `aarch64-apple-darwin`.
- Verified commands: `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`, `cargo build --workspace`, `cargo run -p scriptcraft`, and `cargo clippy --workspace --all-targets -- -D warnings`.
- Semantic model, editing, layout, persistence, exports, and visible UI remain later-WO scope.

## WO-006 verified persistence facts

- `crates/scriptcraft-core/src/persistence.rs` is headless and contains no direct filesystem calls in its pure serialization functions; filesystem access is behind [`FileSystem`](../../crates/scriptcraft-core/src/persistence.rs).
- The native file content is the existing versioned JSON envelope, bounded by `MAX_JSON_BYTES`; the host currently uses `scriptcraft.json` and the core recovery prefix `.scriptcraft-recovery-<revision>.json` with a maximum of three snapshots.
- Recovery corruption is ignored during discovery and candidates are offered by revision; no recovery path silently overwrites the named project.

## WO-004 verified spacing facts

- The fixed screenplay layout contract was measured without changing its production inputs: `612 × 792` units, margins top/bottom/left/right `72/72/90/72`, nominal `55` lines per page, seven-unit character cells, and twelve-unit line boxes.
- Structural layout, desktop viewport-transform, caret, and PDF coordinate regressions pass. Viewport zoom remains a presentation transform and is not an input to print pagination or source mappings.

Evidence: [`WO-004-screenplay-spacing-contract.md`](../initiatives/2026-10-25-ui-quality-of-life/work-orders/WO-004-screenplay-spacing-contract.md), [`screenplay-core.md`](components/screenplay-core.md).

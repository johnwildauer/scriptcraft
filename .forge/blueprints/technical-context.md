# Technical Context — ScriptCraft

## Document status

- **Status:** `draft` — initialized technical baseline; independent blueprint review not performed
- **Owner:** Forge initialization agent
- **Last updated:** 2026-10-30
- **Evidence basis:** observed repository inventory plus inferred/desired architecture from `.analysis`
- **Completion evidence:** `TODO: link fresh-context blueprint review and verified workspace baseline after the foundation initiative`

## Evidence classification

- **Observed at initialization (2026-10-25):** `.analysis/`, `.forge/`, `apps/`, and `crates/` existed; the initial inventory returned no files under the app/core paths. Git was initialized on `master` with no commits and no known remote at that time. This is a historical baseline, not the current repository state.
- **Current observed:** The workspace contains the `scriptcraft` desktop app and `scriptcraft-core`; the app uses eframe/egui 0.29, and the core remains window-independent. UI Polish WO-001–004 implementation evidence and separate passing validations establish selected page geometry/scale, status/appearance, timer, and Focus Mode behavior. Those artifacts do not establish native OS notification delivery, native fullscreen completion/restoration, native visual appearance, or the status-bar discrepancy for AC-WRITING-005.6.
- **Inferred:** The analysis represents product direction and useful architecture context; it is not evidence for behavior absent from current source or validated work-order records.
- **Desired:** Continue to preserve a small dependency-directed Rust core, desktop application shell, headless test path, semantic model, deterministic layout, native persistence/recovery, and explicit export adapters.

## Repository map

| Area | Path/signpost | Responsibility / confidence |
|---|---|---|
| Forge workflow | [`.forge/`](../) | Durable process, requirements, blueprints, and future initiative records — observed |
| Product research | [`.analysis/`](../../.analysis/) | Architecture and MVP research inputs; not executable source — observed |
| Desktop app | [`apps/scriptcraft/`](../../apps/scriptcraft/) | Runnable native app package; eframe/egui shell, editor, status and platform adapters — observed |
| Reusable core | [`crates/scriptcraft-core/`](../../crates/scriptcraft-core/) | Window-independent Rust library for semantic project, editing/session, layout, persistence and exports — observed |
| Desktop UI Polish modules | [`apps/scriptcraft/src/`](../../apps/scriptcraft/src/) | Toolbar, status bar, document rendering, appearance, timer and fullscreen-state reconciliation — source observed; selected WO-001–004 behavior independently validated |
| Test boundary | Workspace app/core unit and integration tests | Headless core and rendered-egui regressions; native OS smoke remains separate and is not inferred from rendered tests |

## Runtime and deployment topology

The observed topology is a native Rust `scriptcraft` desktop process using
eframe/egui above the reusable, window-independent `scriptcraft-core` library.
The desktop owns input/IME, per-window presentation and status state, file/dialog
adapters, and rendering; semantic edits, fixed-profile layout, persistence, and
export contracts remain in the core. PDF consumes fixed print layout. The
workspace/package topology is verified; application packaging/distribution and
some native platform behaviors remain unknown or unverified.

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
| Desktop host | eframe/egui native desktop application | `apps/scriptcraft`, eframe/egui 0.29; native behavior depends on backend/platform | observed in manifest/source; native capability evidence is feature-specific |
| Core boundary | `scriptcraft-core` reusable library | No UI/window dependency; headless-capable | observed in workspace/source |
| Native data | Versioned UTF-8 JSON-like owned project | Extension/schema not selected | desired / provisional |
| Layout | One bounded named/versioned screenplay page profile initially | `us_letter_screenplay` v1: 72-point/inch US Letter (`612 × 792` points), margins top/bottom/left/right `72/72/108/72`, seven persisted role boxes, 12-point Courier Prime default (regular face bundled in desktop builds), 7-unit fixed character cell, 12-unit same-element advance and 12-unit boundary gap; screen/PDF/caret consume shared layout and viewport scale is presentation-only. PDF retains Type1 family mapping and does not embed the bundled TTF. | fixed profile verified; font renderer updated by WO-007 |
| Export | PDF plus deliberately lossy plain text | Exact library/contract unknown | desired / provisional |

## Cross-cutting concerns

- **Configuration and secrets:** No current application configuration is observed. Desired project files exclude UI/session state; secret policy is not applicable to the initial local MVP and remains unknown for future integrations.
- **Authentication/authorization:** None desired for the single-user local MVP; MCP/remote control is out of scope.
- **Errors and retries:** Desired typed/bounded errors, transactional in-memory edits, failed-save preservation, and explicit recovery candidates. Exact error types are not implemented.
- **Logging and observability:** The desktop exposes bounded, privacy-safe status/Diagnostics categories and counters; it is not a source of screenplay data. Broader logging policy remains unspecified.
- **Persistence and migrations:** Native schema version `1` uses the existing bounded deterministic JSON model; parsing rejects malformed/unsupported versions. The core exposes byte operations and an injected filesystem boundary, while the host supplies filesystem paths. Saves use sibling-temp replacement and recovery retains three revision-named candidates.
- **Testing strategy:** Headless core tests and rendered-egui application tests coexist. Required workspace commands and native smoke policy are recorded in `.forge/project-manifest.md`; rendered tests do not establish native OS behavior.

## Boundary inventory

- [`containers/scriptcraft-desktop.md`](containers/scriptcraft-desktop.md) — desired runnable desktop application boundary; exact package and framework require discovery.
- [`components/screenplay-core.md`](components/screenplay-core.md) — desired reusable semantic/session/layout/persistence capability.
- [`features/screenplay-writing-loop.md`](features/screenplay-writing-loop.md) — technical composition of the stable MVP feature.

## Architectural risks and unknowns

- The empty app/core paths and unresolved framework were facts of the 2026-10-25 initialization snapshot only; current package and framework facts are recorded above.
- Native notification delivery, Focus Mode native completion/restoration, OS theme propagation, and native visual smoke are not established by current WO-001–004 evidence.
- Desktop glyph advances do not determine pagination: the fixed seven-unit character cell owns layout and the supplied regular Courier Prime face is bundled in the app. PDF still uses a non-embedded Type1 family mapping, so downstream viewer substitution may vary; do not claim identical PDF glyph appearance without a separately approved embedding change.
- A general document model could reintroduce Word-specific breadth; keep the semantic screenplay model explicit and small.
- Recovery snapshots are not the same as atomic saves; both need separate tests and evidence.
- Incremental pagination, collaboration, interchange, MCP, and web support remain deferred until a demonstrated requirement exists.
- Independent requirements and blueprint reviews have not occurred; this baseline must not be treated as reviewed or validated.

## Revision history

| Date | Change | Evidence/rationale |
|---|---|---|
| 2026-10-25 | Replaced technical-context template with observed/inferred/desired baseline and only necessary MVP boundaries | Workspace inventory plus `.analysis` architecture proposal; no source implementation claimed |
| 2026-10-25 | Recorded WO-001 Cargo workspace and native headless seam | `Cargo.toml`, `apps/scriptcraft`, `crates/scriptcraft-core`, native `aarch64-apple-darwin` target, and passing baseline commands |
| 2026-10-30 | Distinguished the historical empty-workspace baseline from current app/core topology and selected desktop behavior | Current workspace/manifests/source plus separate UI Polish WO-001–004 passing validations; native notification/fullscreen/visual evidence remains explicitly limited |

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

- The fixed screenplay layout contract uses inch-defined values converted at the format boundary: `612 × 792` points, margins top/bottom/left/right `72/72/108/72`, nominal `55` line capacity, seven-unit character cells, twelve-unit same-element line boxes, and twelve-unit boundary gaps. The profile name/version and bounded explicit values are native-serialized; viewport zoom remains presentation-only.
- Structural layout, desktop viewport-transform, caret, and PDF coordinate regressions pass. Viewport zoom remains a presentation transform and is not an input to print pagination or source mappings.

Evidence: [`WO-004-screenplay-spacing-contract.md`](../initiatives/2026-10-25-ui-quality-of-life/work-orders/WO-004-screenplay-spacing-contract.md), [`screenplay-core.md`](components/screenplay-core.md).

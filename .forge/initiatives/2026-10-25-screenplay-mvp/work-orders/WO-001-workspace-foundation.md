# Work Order: WO-001 — Workspace Foundation

## Metadata

- **Status:** `complete`
- **Assignee:** Workspace-foundation implementer (one fresh session)
- **Phase/order:** 1 of 8
- **Initiative:** [`../2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md)
- **Generated filename:** `WO-001-workspace-foundation.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-001.md`](../validations/validation-WO-001.md)
- **Fixes:** `none`
- **Implementer session/agent ID:** `code-session-2026-10-25-wo-001`
- **Session exception:** `none`
- **Design approval:** Human approval recorded in the initiative plan and execution log on 2026-10-25; independent design review passed with warnings.

## Summary

Establish the smallest verified Cargo workspace and runnable desktop/headless seams in `apps/scriptcraft` and `crates`. This WO discovers and records the framework, package graph, first desktop target, commands, and dependency isolation without implementing product features.

## In scope

- Root workspace/package structure, app binary, reusable crate boundary, verified Cargo commands, and minimal startup/smoke seams.
- A dependency check proving core-facing crates do not depend on UI/window types; resolve only foundation-level unknowns.
- Update technical blueprint/manifest only where authorized by discovered facts.

## Out of scope

- Semantic screenplay behavior, editing, layout, persistence, exports, broad UI, DOCX/FDX, collaboration, MCP, planning, or web/multi-platform polish.

## Requirements

- **NFR-WRITING-002:** “Core editing, layout, and persistence behavior shall be exercisable without a desktop window for deterministic tests.”

## Blueprints

- [`technical-context.md`](../../../blueprints/technical-context.md) — verify exact package placement, runtime topology, and commands.
- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md) — establish the native desktop boundary.
- [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md) — preserve the reusable/window-independent core direction.

## Implementation plan

1. Inspect the empty observed paths and create the verified Cargo workspace/package graph with `apps/scriptcraft` and reusable support under `crates`.
2. Select and isolate the smallest desktop framework/runtime seam; provide a headless entry/test seam and record exact commands and first target.
3. Run baseline format/check/test/build/run checks and record results; update durable docs only for verified choices.

## Files and systems

- **Create:** root Cargo manifest, app/package manifests, minimal Rust module/source and baseline tests/configuration.
- **Update:** [`project-manifest.md`](../../../project-manifest.md) and relevant blueprint documents only if the actual discovery resolves their recorded unknowns.
- **Avoid changing:** requirements acceptance wording and all feature implementation boundaries.

## Validation and E2E acceptance tests

- **Automated checks:** Exact verified Cargo format/check/test/build commands discovered by this WO.
- **Manual/external checks:** Start the desktop seam and confirm a minimal visible startup path if the selected framework supports it; record unavailable checks as evidence.
- **Acceptance test:** Given a clean checkout on the initiative branch, when the verified baseline commands run, then the workspace builds/tests and `apps/scriptcraft` has a runnable desktop/headless seam with no UI dependency pulled into the reusable core.

## Durable-document impact

- **Requirements impact:** `verify unchanged`
- **Blueprint impact:** `update`
- **Affected documents/IDs:** [`technical-context.md`](../../../blueprints/technical-context.md), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md), [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md)
- **Rationale and evidence:** The WO discovers package/framework/command facts and must align the provisional technical baseline with observed workspace evidence.
- **Authorized documentation changes:** Record only verified Cargo members, framework isolation, target, and commands; do not claim feature behavior.

## Completion checklist

- [x] Implementation is complete within scope.
- [x] Tests/checks and required gates have evidence.
- [x] Durable-document impact classification is accurate.
- [x] Execution log is appended with dispatch/completion IDs.
- [x] Work order is `in-review`; completion requires a passed fresh validation.

## Implementation and validation evidence

- **Completion session/agent ID:** `code-session-2026-10-25-wo-001`
- **Changed files:** [`Cargo.toml`](../../../Cargo.toml), [`Cargo.lock`](../../../Cargo.lock), [`rustfmt.toml`](../../../rustfmt.toml), [`.gitignore`](../../../.gitignore), [`apps/scriptcraft/Cargo.toml`](../../../apps/scriptcraft/Cargo.toml), [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs), [`crates/scriptcraft-core/Cargo.toml`](../../../crates/scriptcraft-core/Cargo.toml), [`crates/scriptcraft-core/src/lib.rs`](../../../crates/scriptcraft-core/src/lib.rs), [`project-manifest.md`](../../../project-manifest.md), this work order, the initiative execution log, and the three affected blueprints. `target/` is generated and ignored.
- **Verified package graph:** workspace members `apps/scriptcraft` (`scriptcraft` binary) and `crates/scriptcraft-core` (`scriptcraft-core` library); the app depends on the reusable crate; no UI/window dependency is present.
- **Framework/target decision:** No desktop framework dependency selected in WO-001; the smallest native Rust/headless seam is used. `rustc -vV` recorded `rustc 1.99.0` on native target `aarch64-apple-darwin`. Visible-window smoke is unavailable and is recorded as deferred, not passed.
- **Commands and exact results:**
  - `cargo fmt --all -- --check` — initially failed because rustfmt required final blank-line removal; `cargo fmt --all` applied the formatter, then the same check passed with exit code 0.
  - `cargo check --workspace` — passed, exit code 0.
  - `cargo test --workspace` — passed, exit code 0; 1 core unit test passed, app tests 0, doc-tests 0.
  - `cargo build --workspace` — passed, exit code 0.
  - `cargo run -p scriptcraft` — passed, exit code 0; output `scriptcraft foundation ready`.
  - `cargo clippy --workspace --all-targets -- -D warnings` — passed, exit code 0.
- **Independent validation:** [`validation-WO-001.md`](../validations/validation-WO-001.md) passed in fresh session `validation-session-2026-10-25-wo-001`; WO-001 is complete.
- **Follow-ups:** Preserve the design-review warnings in the initiative records; resolve framework/package contract changes before dependent WOs, keep core UI-independent, record validation ownership/evidence, and reconcile WO-006 durable-document classification before WO-006.

# Work Order: WO-012c — Retire legacy project identity and metadata

- **Status:** `implemented`; implementer checks passed. No independent validator dispatched under the initiative-specific user direction.

## Outcome and scope
Remove ScriptCraft project ID/schema and title/author/contact/timestamp metadata from the canonical FDX document model, without deleting foreign FDX metadata or weakening element/source identity.

## Components and implementation
- Update [model and JSON adapter](../../../../crates/scriptcraft-core/src/lib.rs), [preservation](../../../../crates/scriptcraft-core/src/preservation.rs), [desktop New](../../../../apps/scriptcraft/src/main.rs), [toolbar](../../../../apps/scriptcraft/src/toolbar.rs), FDX projections, test constructors and diagnostics. Internal element keys remain collision-free across body/title, anchors and undo; they are not promised to survive reopen. Preserve foreign IDs/DocumentRef/XRef, dates, title-page paragraphs and all unknown values untouched. Do not infer title/author from paragraph positions.
- Keep schema/version handling local to the transitional JSON adapter if still required. Keep the existing public JSON path usable only when it can round-trip its original values exactly; retain an isolated adapter or reject incompatible operations clearly and transactionally. No automatic legacy migration and no deleting existing files. New-document constructors must not fabricate unsaved metadata.
- Keep profile extraction for WO-012d and FDX cutover disabled; adjust temporary writer compatibility checks without allowing loss of still-live legacy state.

## Acceptance and checks
- New/imported FDX model carries no legacy project identity/metadata requirement; foreign metadata and title source survive no-op projection. Element identity collisions, anchors, selection, undo and clean-state behavior remain valid. Nondefault JSON input is retained by the transitional adapter or safely rejected, never silently saved after loss.
- Run `cargo test -p scriptcraft-core`, `cargo test -p scriptcraft`, `cargo test --workspace`, and applicable format/lint/build checks; record results. Test new desktop construction without placeholder metadata and privacy-safe diagnostics.

## Risks and dependency
Depends on WO-012b and WO-003/004/011. One attributable commit with implementer checks only. Risk: source identities or authored title data mistaken for app IDs; do not remove them. Hand model and transitional adapter to WO-012d.

## Implementation decisions and checks — 2026-10-31

- Removed ScriptCraft project ID, project schema version, and title/author/contact/timestamp metadata from the canonical `Project`; the New constructor no longer accepts or fabricates these values. Element IDs remain unique across title page and body and continue to support anchors, editing, and undo.
- FDX source data remains preservation-owned. FDX projection no longer synthesizes/compares retired ScriptCraft identity or metadata; the writer retains source-owned foreign metadata and rejects JSON-origin projects while their legacy state has no FDX home. Unsupported source fields and title-page structures remain unmodified by the no-op projection.
- Kept the legacy JSON fields in a crate-private, JSON-adapter-only state on JSON-origin projects. The adapter retains identity/schema/metadata for exact legacy round trips, rejects unsupported richer model state, and now rejects unknown or duplicate JSON object fields rather than silently discarding them. New and FDX-origin projects have no such compatibility state; JSON serialization returns `UnsupportedJsonState` until the later native cutover. This protects the live model and destination rather than inventing defaults or losing values.
- Updated desktop New/toolbar construction and transitional JSON-dependent tests. Added a fixed legacy JSON fixture for positive exact-round-trip coverage. Added tests for foreign FDX metadata preservation, JSON-source refusal at the FDX writer boundary, new-document diagnostics privacy, cross-domain element ID uniqueness, and unknown/duplicate legacy JSON fields.
- `cargo test -p scriptcraft-core` — passed (108 unit tests, 39 integration tests; 147 total, 0 failed).
- `cargo test -p scriptcraft` — passed (155 tests, 0 failed).
- `cargo fmt --all -- --check` — passed.
- `cargo clippy --workspace --all-targets -- -D warnings` — passed.
- `cargo test --workspace` — passed (302 tests, 0 failed: 155 desktop unit tests, 108 core unit tests, 39 integration tests, 0 doc tests).
- `cargo check --workspace` — passed.
- `cargo build --workspace` — passed.
- `git diff --check` — passed.
- No manual Final Draft verification or independent validation was performed or claimed. New/FDX-origin projects remain unable to serialize to the transitional JSON path until downstream native cutover work; no JSON migration, sidecar, or private FDX extension was added.

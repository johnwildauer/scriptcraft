# WO-001 — egui 0.34 and Material icons foundation

- **Status:** Implementation submitted 2026-11-04; user review/validation pending
- **Initiative:** [`ui-icons-v1.md`](../ui-icons-v1.md)
- **Depends on:** None
- **Validation owner:** User; no agent-run checks or independent reviewers/validators

## Outcome

Move the desktop application from eframe/egui 0.29 to 0.34 and install egui_material_icons 0.6.0 with both filled and outlined fonts available. Migrate all framework API incompatibilities while preserving current application behavior. Do not change status icons or layout in this order.

## Scope

1. Update [`apps/scriptcraft/Cargo.toml`](../../../apps/scriptcraft/Cargo.toml:8):
   - Change the eframe dependency to the 0.34 release line.
   - Add egui_material_icons 0.6 with its default filled/compressed features plus the outline feature. Keep one compatible egui 0.34 type family across eframe and the icon crate.
   - Leave rfd and the local screenplay-core dependency unchanged unless a concrete dependency-resolution incompatibility requires a separately recorded correction.
2. Update [`Cargo.lock`](../../../Cargo.lock) through Cargo dependency resolution. Lockfile generation is setup, not evidence that compilation, tests, or behavior passed.
3. In the app-creation callback in [`main()`](../../../apps/scriptcraft/src/main.rs:2792), retain [`document_font::install()`](../../../apps/scriptcraft/src/document_font.rs:53), then initialize Material icons once on the same egui context before constructing the app. Preserve all Courier Prime named families and do not replace them with Material fallback fonts.
4. Migrate compiler-facing egui/eframe 0.34 API differences across the desktop modules only as needed: application creation, raw input and viewport state, scroll geometry, text/font handling, AccessKit projection, panels, popups, painter calls, and widget responses. Prefer direct API equivalents and preserve existing geometry and event ordering.
5. Preserve the validated deep-scroll zoom behavior described in [`technical-context.md`](../../../.forge/blueprints/technical-context.md:97). In particular, do not reintroduce post-paint zoom correction or alter the established viewport-positioned rectangle/content-space normalization contract merely because framework source internals changed.
6. Record every compatibility adaptation and any unresolved framework behavior difference in this order's implementation handoff. Do not silently change product behavior to obtain compilation.

## Explicit non-goals

- No status-bar glyph, caption, hover, width, overflow, zoom-control, or ordering changes.
- No toolbar icon substitutions or edits to the WIP toolbar assignment table.
- No core-crate semantic, layout, persistence, or export changes.
- No custom Material font files and no weight-200 claim.
- No broad cleanup, formatting-only rewrite, dependency upgrade beyond the authorized framework/icon path, or new automated-test target.

## Implementation boundaries

- **Primary files:** [`apps/scriptcraft/Cargo.toml`](../../../apps/scriptcraft/Cargo.toml), [`Cargo.lock`](../../../Cargo.lock), and [`main.rs`](../../../apps/scriptcraft/src/main.rs:2792).
- **Compatibility fallout only:** the other modules under [`apps/scriptcraft/src/`](../../../apps/scriptcraft/src/) when an egui/eframe 0.34 API requires it.
- **Avoid:** [`crates/scriptcraft-core/`](../../../crates/scriptcraft-core/), bundled Courier Prime assets, unrelated historical plans, and retired validation/review artifacts.
- Before implementation, the execution owner must establish the required initiative branch under the manifest's branch policy and preserve any pre-existing user work. This plan does not claim a branch was created.

## Structural acceptance

- The desktop manifest and lockfile select eframe/egui 0.34 and egui_material_icons 0.6.0 with filled and outline support, without retaining a second active egui version for the desktop.
- Material fonts initialize after the existing document fonts and before app construction.
- Timer, Location Searching, and Mode Night constants are available for the next order; no status behavior is changed here.
- All compatibility edits are enumerated, with no undocumented changes to input, zoom, save/open, editing, pagination, popup, or export behavior.
- Any behavior difference that cannot be preserved is reported as a blocker for user disposition rather than represented as an intentional improvement.

## Durable records

During implementation:

- Update the framework/dependency observations and user-approved framework state in [`project-manifest.md`](../../../.forge/project-manifest.md:39), preserving the authoritative no-agent-check policy.
- Add a concise desktop icon-foundation paragraph to [`technical-context.md`](../../../.forge/blueprints/technical-context.md:20): eframe/egui 0.34, egui_material_icons 0.6, filled plus outline initialization after Courier Prime, and Regular-weight rendering.
- Annotate [`icon_assignement.md`](../../../.reference/icon_assignement.md:1) with the selected crate/version/features and the user-approved Regular-weight exception. Preserve the original weight-200 design preference and all WIP assignments.
- Append implementation scope and unresolved evidence to [`ui-icons-v1.md`](../ui-icons-v1.md) and this work order. Do not claim user validation or initiative closeout.

## User-owned review requests

These are handoff requests, not commands run during planning or authorization for agent-run checks:

- Resolve dependencies and run [`cargo check --workspace`](../../../Cargo.toml:1), [`cargo test --workspace`](../../../Cargo.toml:1), and the other desired manifest checks; record exact outcomes separately.
- Launch the desktop app and review typing/editing, scrolling and deep-scroll zoom, native pinch and status zoom, toolbar commands, Open/Save/Save As, timer popup, Focus Mode, Appearance, and Diagnostics for migration regressions.
- Confirm document text still uses Courier Prime and that registering Material icons did not replace or unexpectedly change document typography.

## Implementation handoff — 2026-11-04

Implemented on `dev/2026-11-04-ui-icons` only. The desktop manifest now uses
eframe 0.34 and egui_material_icons 0.6.0 with `outline` enabled alongside the
crate's default filled/compressed features. Cargo dependency resolution
generated `Cargo.lock`, resolving 437 packages and selecting eframe 0.34.3 and
egui_material_icons 0.6.0. This is dependency setup, not compile or test
evidence.

The app-creation callback registers Courier Prime first, initializes Material
fonts on that same context, then constructs the app. Desktop-only API
adaptations made for the 0.34 transition:

- Eframe rendering now enters through `App::ui`, forwarding to the existing
  app update logic.
- Raw scrolling checks nonzero `Event::MouseWheel` deltas while retaining the
  smooth-scroll invalidation path; the existing event/prepaint correction order
  is unchanged.
- Courier Prime `FontData` entries are stored in `Arc` as required by the new
  font definitions; Material registration follows without replacing named
  document families.
- Text layout jobs are made mutable owned values before wrapping changes and
  are laid out through the painter API. Toolbar/status text measurement uses
  the painter's layout methods.
- AccessKit label nodes use the current label setter; widget enabled state uses
  the response accessor; placed-row bounds are represented through row
  position/size, using copy-on-write for the underlying row.
- Page border painting supplies the new stroke kind while preserving the
  centered stroke.

The deep-scroll zoom coordinate normalization, prepaint compensation,
Courier-Prime document typography, popup behavior, FDX behavior, and headless
core were not intentionally changed. No status icon or layout work was done.
The observed IDE migration diagnostics were addressed, but compilation and all
behavior remain unverified; other migration issues may remain. No build, test,
`cargo check`, lint, formatting check, manual verification, or review was run.
Validation and review remain user-owned. WO-002/WO-003 remain out of scope and
unstarted. No user acceptance is recorded.

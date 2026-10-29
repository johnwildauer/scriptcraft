# Work Order: WO-007 — Courier Prime Default Document Font

## Metadata

- **Status:** `complete` — limited aggregate directional-alignment check passed; this is not comprehensive code validation.
- **Assignee:** Dedicated fresh coding implementer, separate from WO-005/WO-006; actual host dispatch ID `01a1861e-76e0-7509-8b2c-c4e7dfc13367` (user-supplied; host dispatch ID only). No internal session ID is available or inferred.
- **Phase/order:** 3 of 4 in the user-directed follow-on batch.
- **Initiative:** [`status-bar-v2.md`](../status-bar-v2.md)
- **Dependencies:** WO-006 implementation handed off `in-review`; continue sequentially after it.
- **Next:** WO-008 after this implementation handoff.
- **Fixes:** none; new scoped change.

## Summary

Use the user-supplied regular `Courier Prime.ttf` as the default face for newly created document text. Register/use supplied bold, italic, and bold-italic variants where current rendering/semantic architecture has a genuine matching style; do not invent a formatting system. Preserve existing user files, font license, saved project/layout/source-map behavior, and PDF/export invariants unless inspection proves the current architecture requires a narrow change.

## Observed baseline and asset ownership

- Current core `PageProfile::default()` selects `Courier`; app renderer uses egui built-in monospace; profile validation already accepts `Courier Prime`; PDF maps that profile family to `/CourierPrime` in the existing Type1 resource path.
- User-supplied assets under [`fonts/CourierPrime/`](../../../../fonts/CourierPrime/) are `Courier Prime.ttf`, `Courier Prime Bold.ttf`, `Courier Prime Italic.ttf`, `Courier Prime Bold Italic.ttf`, `Read me.txt`, `LICENSE/OFL.txt`, and `LICENSE/OFL-FAQ.txt`. The license identifies SIL OFL 1.1 and reserved font name Courier Prime.
- Workspace/app manifests establish the Cargo package boundary; no dedicated font dependency was found. Inspect actual build/bundle ownership before selecting an embedding/loading mechanism.

## In scope

- Read supplied fonts, README/license and inspect actual font registration, document renderer, profile defaults, packaging/build path, semantic formatting, and PDF/export mapping before editing.
- Make Courier Prime the default for newly created document profiles and have desktop document rendering use the bundled regular face at existing profile size. Preserve explicit stored supported families and existing serialized profile values on open/save; no silent migration or rewrite of existing projects.
- Register/use supplied weight/slant variants only where existing role/style information requests them; otherwise do not invent semantics or imply a style the model does not express. No new font picker or management UI.
- Preserve fixed character/page geometry; test text advances, wrapping, carets, selection/IME rectangles, page count, pagination, and source-position mapping. If metrics conflict with fixed cell geometry, document evidence and choose the smallest compatible rendering method without changing canonical text/source coordinates unless an architecture-required change is evidenced and within scope.
- Preserve PDF/export determinism and output contract unless inspection proves a narrow change is required by actual architecture; if required, explain why and test default/alternate output. Do not embed/transform font files unless necessary and permitted by OFL.
- Preserve every supplied asset byte-for-byte and at current path; carry copyright/OFL 1.1 notices with any redistributed copy. Never overwrite, rename, delete, relocate, or clean up `fonts/CourierPrime/`, including `.DS_Store` if present.

## Out of scope

- Modifying/removing/replacing user-supplied font files/license/README; downloading fonts; changing license/reserved name; dropping notices from a distribution.
- Arbitrary per-element formatting, new role semantics, font choice UI, font databases, schema migration, or changing existing saved profiles on load.
- Unnecessary core geometry/profile/schema changes, source-map/pagination changes, PDF/export redesign, changes to document text, or unrelated UI.

## Requirements and durable-document impact

- Review [`REQ-WRITING-LOOP-001`](../../../requirements/features/screenplay-writing-loop.md:44) and profile/layout invariants. Update human-readable format rules and applicable core, feature, and desktop blueprints currently stating the `Courier` default or built-in fallback to describe new-project Courier Prime and retained compatibility.
- **Records reviewed:** [`format-rules.md`](../../../../.forge/initiatives/2026-10-25-editor-geometry-corrections/format-rules.md:5), [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:151), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:92), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:43), and [`technical-context.md`](../../../blueprints/technical-context.md:61). Update each stale default/fallback statement to distinguish bundled app rendering from the existing non-embedded PDF Type1 mapping; do not claim measured advances or PDF font embedding. Requirements are verified unchanged.
- Keep license statements intact and record a precise update/`verify unchanged` disposition for each relevant document.

## Implementation files

- **Allowed product paths:** app font registration/renderer under `apps/scriptcraft/src/` (including `main.rs` or a small app-local font module); [`crates/scriptcraft-core/src/lib.rs`](../../../../crates/scriptcraft-core/src/lib.rs) only if the new-project default requires it; [`crates/scriptcraft-core/src/exports.rs`](../../../../crates/scriptcraft-core/src/exports.rs) only if inspected PDF architecture genuinely requires a bounded change; packaging/build files only if actual bundling requires them and the diff is explained.
- **User-owned read-only inputs:** all existing files under `fonts/CourierPrime/`. If packaging requires copies, preserve originals byte-for-byte and include copyright/OFL 1.1 with redistributed copies.
- **Allowed durable records:** applicable blueprint/format-rule updates, this WO, [`status-bar-v2.md`](../status-bar-v2.md), and [`execution-log.md`](../execution-log.md). Verify applicable requirements unchanged; do not edit requirement text absent a newly required criterion.
- **Avoid changing:** project text, unrelated profile/layout fields, PDF behavior without evidence, dependencies/lockfile absent necessity, other assets, WO-004/historical records, `.DS_Store`, and unrelated files.

## Measurable acceptance and focused regressions

1. New projects store exact supported family name `Courier Prime`; round-trip/open tests prove existing explicit profile names are preserved without rewrite/migration.
2. Actual document rendering selects the supplied regular font (test font data/family registration used by renderer, not only a string). Where existing semantics support bold/italic, tests prove matching supplied variants are used; no new style semantics are invented.
3. Representative screenplay-role fixtures preserve canonical UTF-8 text, fixed profile/layout coordinates, page/source mapping, caret/selection/IME geometry, and deterministic page count at current base size. Any architecture-required metric change must include measured evidence and preserve source-coordinate correctness.
4. Save/open and text/PDF exports retain existing invariants. If no export change is needed, assert current behavior remains; if a change is required, prove deterministic default/alternate output and document the exception. No data loss or dirty/undo mutation.
5. Every supplied font/license/README file remains present and byte-identical; any distribution contains OFL notices and does not sell the font alone or use the reserved name for a modified font.

## Implementation evidence — 2026-10-29

- **State/dispatch:** WO-007 transitions `in-progress` → `in-review` after the user-authorized sequential implementation and required checks. The sole user-requested aggregate alignment check remains pending after WO-008. Actual implementer host dispatch ID `01a1861e-76e0-7509-8b2c-c4e7dfc13367` was supplied by the user and reconciled; no internal session/agent ID is available or inferred. WO-008 proceeds after this implementation handoff.
- **WO-006 dispatch reconciliation:** The user supplied actual WO-006 host dispatch ID `01a1860d-a80d-7069-8ad5-bbdb34b1492a`; it is recorded in WO-006, the v2 plan, and execution log as a host dispatch ID only. No internal session ID was invented.
- **Implementation:** `PageProfile::default()` now uses exact family `Courier Prime`; existing serialized `Courier`, `Courier New`, and `Courier Prime` names remain accepted and round-trip unchanged without schema migration. The desktop startup registers the supplied regular face in a named egui family and the document painter selects it from the persisted profile for page text and IME preedit. Compile-time `include_bytes!` embeds the supplied four font variants into the desktop executable; bold/italic faces are registered, but no screenplay role exposes weight/slant semantics, so they are not selected and no formatting semantics were invented. Legacy explicitly stored `Courier`/`Courier New` retain the established monospace renderer path. Per-character fixed-cell painting, source ranges, page layout, caret/selection/IME geometry, and viewport scaling remain the geometry authority.
- **Export:** PDF continues to consume the same fixed layout/profile and its existing deterministic Type1 family mapping; the default profile maps to `/CourierPrime`. The supplied TTF is not embedded in PDF, and no PDF contract redesign was justified. This work makes the face available in the app build, not a claim of identical downstream PDF glyph appearance.
- **Focused regressions:** App tests verify exact registered bytes/family keys for all four font files, selected profile-family mapping (including legacy selection), and actual emitted document text galleys selecting Courier Prime while retaining seven-unit fixed-cell spacing and source-byte ranges. Core tests verify the exact new-project default, explicit supported-family JSON round trips, and deterministic default/alternate PDF mapping.
- **Focused command results:** `cargo fmt --all -- --check` passed; focused font renderer/registration tests passed (3 app tests); `new_project_has_valid_start` passed (1 core test); `supported_profile_fonts_round_trip_without_rewriting` passed (1 core test); PDF export tests passed (3 core tests). The full package/workspace runs below include these again.
- **Full command results:** `cargo test -p scriptcraft` passed (114); `cargo test -p scriptcraft-core` passed (45 unit + 37 integration; 0 doc tests); `cargo check --workspace`, `cargo test --workspace` (114 app + 45 core unit + 37 integration = 196, 0 failures), `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` all passed. A post-build byte scan confirmed `target/debug/scriptcraft` contains byte-identical copies of all four supplied TTF files.
- **Assets/license:** All supplied font, README, OFL 1.1, OFL FAQ, and `.DS_Store` files remained present under `fonts/CourierPrime/`; pre/post SHA-256 snapshots matched and no font-directory path was edited. The copyright notice and reserved name remain intact. No separate distribution/package target exists in this checkout, so package-level license inclusion was not exercised; any future redistribution of the embedded TTF must carry the supplied OFL notice. The font was not modified.
- **Durable documents:** Updated [`format-rules.md`](../../../../.forge/initiatives/2026-10-25-editor-geometry-corrections/format-rules.md), [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md), the feature and desktop blueprints, and [`technical-context.md`](../../../blueprints/technical-context.md) for the new-project default, bundled desktop face, fixed-cell geometry, and non-embedded PDF boundary. `REQ-WRITING-LOOP-001` and the current writing-loop requirements were verified unchanged; no requirement file was changed by WO-007.
- **Native/evidence limitation:** Native desktop visual/font smoke was not performed and is not claimed. The implementation is supported by headless rendered-egui tests and built-executable byte inspection only. PDF font embedding/visual parity was not tested or changed.
- **Changed paths for WO-007:** `apps/scriptcraft/src/main.rs`; new `apps/scriptcraft/src/document_font.rs`; `crates/scriptcraft-core/src/lib.rs`; `crates/scriptcraft-core/src/exports.rs`; the five listed durable format/blueprint documents; this WO; the v2 plan; and execution log. Pre-existing WO-004/005/006 product and evidence work, requirements, all user-owned assets, and unrelated worktree changes remain preserved. The index is empty; no stage, commit, reset, stash, merge, cleanup, or agent dispatch occurred.

## Checks and handoff evidence

- Run focused profile/round-trip/render-font/layout/source-map/export tests, `cargo test -p scriptcraft`, and relevant `cargo test -p scriptcraft-core` suites.
- Run project gates: `cargo fmt --all -- --check`; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`.
- Inspect package contents and exact status/bytes of protected fonts; record native font visual-smoke availability honestly.
- **Acceptance test:** Given a new document and supplied font bundle, renderer uses regular Courier Prime, applies only architecture-supported variants, preserves project compatibility and screenplay geometry/source/export invariants, and redistributes required OFL notices without modifying user-owned inputs.

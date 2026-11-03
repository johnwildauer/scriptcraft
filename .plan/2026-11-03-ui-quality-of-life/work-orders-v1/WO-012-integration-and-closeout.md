# RS-WO-012 — Integrated equivalence and initiative closeout

- **Status:** Integration scenario and closeout documentation authored 2026-11-03; implementation coverage unrun; user review, validation, measurements, and acceptance pending
- **Initiative:** [`ui-quality-of-life-v1-render-speed.md`](../ui-quality-of-life-v1-render-speed.md)
- **Depends on:** [`RS-WO-001`](WO-001-performance-diagnostics.md) through [`RS-WO-011`](WO-011-change-scoped-edit-validation.md)
- **Validation owner:** User; no agent-run checks or independent validators

## Objective

Prove the combined batch preserves layout, editing, source, and export behavior while meeting the initiative's structural responsiveness invariants, then record user-measured before/after results and remaining limitations.

This is an integration and documentation order, not permission for additional optimization.

## Implementation and closeout steps

1. Author one integrated representative 120-page user scenario through open, idle, scroll, zoom, selection, ordinary typing, split/join, role changes, derived refresh, undo/redo, Save/Save As, reopen, PDF export, and plain-text export. The user runs it; agents do not.
2. Author cached-versus-fresh layout and preservation/editing equivalence coverage. Deep-clone and complete-validation paths are controls where existing APIs and tests expose them; do not claim a comparison that is not actually authored.
3. Map the authored coverage to the complete structural matrix:
   - identical layout object for unchanged scroll and caret blink;
   - visible-plus-fixed-overscan page painting only;
   - visible line/span text work only;
   - unchanged-element wrap reuse and bounded cache retention;
   - indexed caret/hit/navigation queries;
   - zero unchanged-revision derived clones/reconciliations;
   - affected-scene-only derived candidates;
   - copy-on-write preservation snapshots;
   - change-scoped ordinary-edit lookup and validation with full global gates intact.
4. Record authored coverage and user results for every predecessor order in the initiative document.
5. Complete the baseline/result table with build profile, page/element dimensions, operation, before/after observations, counters, user, and date. Do not invent or infer user measurements.
6. Finalize responsiveness requirements in [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md:617), the measured-cache clarification in [`product-overview.md`](../../../.forge/requirements/product-overview.md:151), and final ownership/boundary descriptions in [`technical-context.md`](../../../.forge/blueprints/technical-context.md:43).
7. Record unresolved bottlenecks by phase and decide whether any further work requires a new initiative.
8. Keep raster page caching, background layout, rope-based canonical text storage, and suffix-incremental pagination outside this batch unless fresh evidence supports separate approval.

## Integration scenario authored for the user

The end-to-end desktop procedure below is an authored scenario, not an executed result. Start with an FDX document representative of the desktop oracle's 120-layout-page fixture (one title page and 119 body pages, 54 known visual lines per body page) and retain a separate untouched copy of the source. Use the same document, viewport, display scale, profile, and operation sequence for every compared run.

1. Open the source and record its page/element dimensions and profile. Confirm open did not rewrite the source. Capture diagnostics at a reset benchmark boundary; inspect only the value-free phase/counter summary.
2. Let the document idle across repeated due refresh ticks. Record the visible page and diagnostics; distinguish same-revision skips from a changed, deferred, or completed refresh. Include a supported scene and a deliberately deferred/unsupported scene where the input document provides them; confirm the deferred scene subtree remains intact while independent supported scenes may reconcile.
3. Scroll first-to-middle-to-last and back using incremental and large scrolls; observe idle/caret-blink frames, page status, and the same-scroll layout identity. Change zoom and horizontal overflow, select text by pointer, reveal the caret, and test visible/offscreen caret and IME placement. Check painted pages/lines/spans remain limited to the editor clip plus fixed overscan while full page geometry/status stays available.
4. Type ordinary text at the beginning, middle, and end; include Unicode and a selection boundary adjacent to an AllCaps expansion when the source has that style. Exercise Enter/split, join, role changes, undo, and redo. Confirm exact canonical casing/source mapping, focus/IME event behavior, role transitions, and same-frame rendering. Observe wrap builds/reuses, fixed-cell galley builds/reuses, indexed query/probe counts, scoped-validation attempts/fallbacks, and preservation share/detach counts where exposed; do not infer speed from counts alone.
5. Trigger or wait for derived refresh after a relevant edit, then repeat idle ticks. Check normalized Character/scene/Cast List output, scene-local deferral, undo/redo behavior, and no repeated unchanged-revision reconciliation. Save, Save As to a new destination, reopen both saved output and the untouched original copy, and compare semantic content, identities where promised, supported derived data, opaque source nodes, title-page data, and preservation-owned FDX structure. Confirm failed-save safety only through the user's selected failure scenarios and preserve a backup before any destructive manual failure case.
6. Export PDF and plain text. Compare fixed print pagination/layout and role/display behavior, confirm the export loss notice and HiddenText omission as applicable, and verify exports did not change the native source, dirty state, selection, or history.
7. Repeat the same observable operation sequence against the baseline reference and record observations and structural counters for every row in the result table. Record a limitation or `not observable` rather than filling gaps by inference.

### Authored coverage map (not execution evidence)

| Integrated concern | Authored evidence to review/run | Boundary / status |
|---|---|---|
| Long-document layout/edit/history/export/source round-trip | [`wo012_integrated_render_preservation.rs`](../../../crates/scriptcraft-core/tests/wo012_integrated_render_preservation.rs); builds a deterministic long imported FDX document, checks at least 120 pages, compares fresh and cached complete layouts before/after an ordinary Unicode edit and undo/redo, verifies FDX opaque source retention and export isolation. | Authored; unrun. It uses the codec round-trip, not filesystem Save/Save As and not a desktop interaction test. |
| Actual desktop 120-page open/idle/scroll/selection/input/zoom/refresh work | RS-WO-002 desktop oracle in [`main.rs`](../../../apps/scriptcraft/src/main.rs), including the 120-page imported-FDX fixture and later-order counter assertions. | Authored; unrun. The user scenario above remains necessary for native desktop behavior. |
| Layout identity, viewport paint, visible lines/spans, fixed-cell geometry and Unicode source mapping | WO-002/004/005 desktop oracle plus fixed-cell galley cases in [`main.rs`](../../../apps/scriptcraft/src/main.rs). | Authored; unrun; no timing threshold. WO-005 retains a scalar-paint fallback when the pinned egui/epaint glyph representation is unsupported; that fallback preserves correctness but reduces batching for affected spans. |
| Per-element wrap reuse and cached/fresh full layout structure | WO-002 desktop edit oracle and WO-006 core differential/cache cases in [`layout.rs`](../../../crates/scriptcraft-core/src/layout.rs). | Authored; unrun. Whole-document page placement remains; no fully incremental suffix pagination claim. |
| Indexed caret/hit/navigation equivalence | WO-007 differential reference cases in [`layout.rs`](../../../crates/scriptcraft-core/src/layout.rs) and desktop query counters in [`main.rs`](../../../apps/scriptcraft/src/main.rs). | Authored; unrun. |
| Idle revision gate, changed/deferred scenes, candidate ownership and failure atomicity | WO-008 desktop scheduler oracle and WO-009 core reconciliation cases in [`scene_metadata.rs`](../../../crates/scriptcraft-core/src/scene_metadata.rs), [`cast_list.rs`](../../../crates/scriptcraft-core/src/cast_list.rs), and related tests. | Authored; unrun. WO-009's no-original-byte/no-unrelated-subtree-copy requirement is addressed by WO-010 COW design, not confirmed by execution. No previous-implementation complete-output golden existed; authored assertions are not a generated golden comparison. |
| Snapshot/history/save candidate isolation and managed-source validation | WO-010 preservation-sharing and WO-011 scoped-validation regressions in [`preservation.rs`](../../../crates/scriptcraft-core/src/preservation.rs), [`editing.rs`](../../../crates/scriptcraft-core/src/editing.rs), and existing FDX/history/save suites. | Authored; unrun. WO-010 coverage does not exhaustively compare every edit operation to a prior deep-clone implementation. WO-011 safe fallback retains complete validation when scoped preconditions are not proven. |
| Real native Save/Save As/reopen and desktop presentation | Existing storage, source-retention, and save/reopen suites plus the user procedure above. | Test code authored in prior orders; all unrun. The integrated core test does not substitute for actual filesystem transaction or desktop/FDX inspection. |

The order's phrase “run the integrated scenario” is implemented here as scenario authorship and an explicit user-run procedure. Repository policy in [`project-manifest.md`](../../../.forge/project-manifest.md:73) makes review and validation user-owned, so no scenario, test, build, format/lint check, manual verification, or implementation check was run. Authored coverage is not passing evidence. Where a deep-clone or historical complete-output golden is absent, comparison is explicitly limited rather than fabricated.

## Final acceptance

- [ ] User has reviewed the integrated coverage and work-order implementation, including public API compatibility caveats.
- [ ] User has run and reviewed applicable workspace checks and authored tests; no authored test is claimed passing here.
- [ ] User has completed desktop and FDX/source-preservation checks, including actual Save/Save As/reopen, export, undo/redo, scene deferral, and failure-safety cases.
- [ ] User has compared cached and fresh outputs and confirmed the structural matrix in the integrated scenario.
- [ ] User has recorded before/after observations, counters, build profiles, document dimensions, user/date, and remaining limitations without timing thresholds.
- [ ] User has recorded final acceptance or required remediation in the initiative record.

Every gate above is pending. Implementation and test authorship do not equal validation or acceptance.

## User validation

User-owned sequence (do not treat this authored handoff as execution):

1. Review the pending implementation/API caveats and this coverage map. Preserve the current initiative checkout and its uncommitted work.
2. In the chosen profiles, run the applicable user-owned checks and authored tests, for example `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo build --workspace`, and `cargo check --workspace`. Use `cargo run -p scriptcraft` for a debug desktop session and `cargo run --release -p scriptcraft` for a release desktop session when applicable. Retain command output as user evidence; distinguish authored tests from tests actually run and discover any external gates. No checks have been run for this closeout.
3. Follow the integrated desktop procedure above on the same representative FDX in each profile. Include actual native Save and Save As, reopen, PDF/plain-text exports, supported and deferred derived scenes, and an explicitly safe failure case if selected.
4. For a baseline, use the original `master` reference from which this initiative branch was created, not an assumed post-initiative state. No baseline was captured before implementation. After confirming `master` still identifies the initiative base, a separate user-managed worktree can be created with `git worktree add --detach ../scriptcraft-render-speed-baseline master`; this does not switch, clean, or modify the current checkout. If `master` has moved, substitute the exact original base commit. Do not overwrite the source document; compare identical input copies and operation sequences.
5. Record qualitative observations and the available structural counters in the initiative table. Record build profile, document dimensions, operation, before/after observation, counters, user, and date. Do not add timing thresholds or infer an improvement from counters; leave unavailable values pending.

## Handoff

No implementation order follows automatically. Commit, merge, further optimization, or a new initiative requires the user's separate direction under repository policy. The initiative remains implemented/authored but unreviewed and unvalidated; final acceptance is pending.

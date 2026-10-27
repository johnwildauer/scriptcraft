# Validation — WO-006a Complete Responsiveness Measurement (v2)

## Validation metadata

- **Validation status:** `PASS WITH WARNINGS`
- **Validated work order:** [`WO-006a-complete-responsiveness-measurement.md`](../work-orders/WO-006a-complete-responsiveness-measurement.md:1)
- **Original work order:** [`WO-006-long-document-responsiveness.md`](../work-orders/WO-006-long-document-responsiveness.md:1)
- **Failing validation:** [`validation-WO-006-v2.md`](validation-WO-006-v2.md:1)
- **Initiative:** [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:1)
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `architect-fresh-context-editor-geometry-corrections-v2-WO-006a-validation-2026-10-26`
- **Session exception:** `none`
- **Date:** `2026-10-26`
- **Product-code changes:** none
- **Existing-artifact changes:** none; this file is the new validation artifact

## Validation boundary and comparison basis

This fresh validation assesses only the three authorized corrections: actual-boundary frame/event timing accumulation, benchmark use of the implemented revision/profile cache path rather than an `Arc` clone proxy, and per-case p50/p95 plus frame/event/layout/paint/repaint output. It also checks preserved cache invalidation, privacy, determinism, redraw policy, incremental-pagination deferral, scope, durable impact, and required Cargo gates. It does not assess unrelated v2 work orders, general UI quality, native visual behavior beyond the documented evidence boundary, or future pagination architecture.

The comparison basis was [`05-validation.md`](../../../process/05-validation.md:1), the approved oracle in [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:73), [`design-review-v2.md`](../reviews/design-review-v2.md:146), the original and fix work orders, the failing validation, the v2 execution log, the current source/tests, and the updated blueprints. The fix handoff, current source, focused tests, recorded benchmark output, and gate records are sufficiently consistent for meaningful comparison. No standalone branch diff or command-execution result was available in this repository-reading validation context.

## Findings

### EGC-V2-WO006A-P-001 — Actual frame/event timing boundaries are populated

- **Severity:** pass
- **Evidence:** [`PerformanceDiagnostics`](../../../../apps/scriptcraft/src/main.rs:32) retains only bounded numeric counters/totals. [`eframe::App::update()`](../../../../apps/scriptcraft/src/main.rs:1032) starts a frame [`Instant`](../../../../apps/scriptcraft/src/main.rs:1034) and accumulates elapsed nanoseconds into `frame_nanos` after the frame path at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1137). [`dispatch_surface_events()`](../../../../apps/scriptcraft/src/main.rs:635) starts its own timer and accumulates elapsed dispatch time into `event_nanos` at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:702). The focused regression at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1510) asserts `event_nanos > 0` after dispatch. Layout-build and page-paint timing are accumulated around the cache miss and [`paint_page()`](../../../../apps/scriptcraft/src/main.rs:389) boundaries at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:106) and [`main.rs`](../../../../apps/scriptcraft/src/main.rs:495); repaint requests are counted by [`request_repaint()`](../../../../apps/scriptcraft/src/main.rs:101).
- **Assessment:** The first failing-validation hard stop is resolved at the actual app boundaries, without text retention. The frame/event totals are not merely declared fields or synthetic benchmark labels.

### EGC-V2-WO006A-P-002 — Benchmark exercises the implemented cache path

- **Severity:** pass
- **Evidence:** [`LayoutCache`](../../../../crates/scriptcraft-core/src/layout.rs:63) is a reusable core cache with revision/profile lookup in [`LayoutCache::get_or_build()`](../../../../crates/scriptcraft-core/src/layout.rs:73). The app uses that seam in [`ScriptCraftApp::layout_for_render()`](../../../../apps/scriptcraft/src/main.rs:106), and the benchmark imports and calls the same [`LayoutCache::get_or_build()`](../../../../crates/scriptcraft-core/tests/wo006_responsiveness.rs:101). The cached measurement creates one cache, passes the revision and project to the lookup, and records the returned hit flag; it does not clone an already-built `Arc` as the measured operation. The focused regression at [`wo006_responsiveness.rs`](../../../../crates/scriptcraft-core/tests/wo006_responsiveness.rs:173) proves a build followed by an unchanged hit. The recorded unchanged 180-page comparison is `19.981709 ms` uncached p95 versus `0.001667 ms` actual-cache p95, reported as `100.0%` reduction.
- **Assessment:** The second failing-validation hard stop is resolved. The benchmark now measures the implemented cache lookup/build seam, with sufficient displayed precision for the cached comparison.

### EGC-V2-WO006A-P-003 — Complete per-case benchmark output is present

- **Severity:** pass
- **Evidence:** The benchmark retains exactly `3` warmups and `10` samples at [`wo006_responsiveness.rs`](../../../../crates/scriptcraft-core/tests/wo006_responsiveness.rs:9), covers page cases `1/5/15/50/120/180` at [`wo006_responsiveness.rs`](../../../../crates/scriptcraft-core/tests/wo006_responsiveness.rs:12), and runs beginning/middle/end edits at [`wo006_responsiveness.rs`](../../../../crates/scriptcraft-core/tests/wo006_responsiveness.rs:143). [`print_measurement()`](../../../../crates/scriptcraft-core/tests/wo006_responsiveness.rs:124) prints p50/p95 milliseconds and frames, events, layout builds, cache hits, painted pages, and repaint requests for each case. The assertions at [`wo006_responsiveness.rs`](../../../../crates/scriptcraft-core/tests/wo006_responsiveness.rs:149) verify sampled frame/event/build/paint/repaint counts for edit cases; the unchanged-read assertions at [`wo006_responsiveness.rs`](../../../../crates/scriptcraft-core/tests/wo006_responsiveness.rs:168) verify zero builds and ten cache hits.
- **Assessment:** The third failing-validation hard stop is resolved. The required five counters are present per case; cache-hit detail is additional and relevant to the approved oracle.

## Preserved implementation-alignment checks

| Boundary | Result | Evidence and assessment |
|---|---|---|
| Cache key and invalidation | **PASS** | [`LayoutCache::get_or_build()`](../../../../crates/scriptcraft-core/src/layout.rs:73) compares the session revision and complete persisted `PageProfile` before returning a hit. The app regression changes `line_advance` and then performs an edit, proving profile and revision misses at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1202). Viewport scale remains presentation-only in the app paint path. |
| Privacy | **PASS** | [`PerformanceDiagnostics`](../../../../apps/scriptcraft/src/main.rs:32) and benchmark [`Counters`](../../../../crates/scriptcraft-core/tests/wo006_responsiveness.rs:14) contain numeric aggregate data only. Fixture dimensions, timing summaries, phase labels, and counters are reported; generated document text is not retained or printed. |
| Deterministic geometry, caret/hit/navigation, and export | **PASS by source/evidence alignment** | The cache returns an immutable shared [`ScreenplayLayout`](../../../../crates/scriptcraft-core/src/layout.rs:55) used by rendering and app navigation/hit/caret consumers, while export continues to construct its own deterministic core layout as recorded in [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:101). Existing layout/caret/hit/PDF tests remain the stated determinism boundary. The focused cache regression also verifies identical layout values on build and unchanged hit. |
| Redraw policy | **PASS** | Active input and scroll request repaint through [`handle_surface_input()`](../../../../apps/scriptcraft/src/main.rs:739); focus transitions request a transition repaint at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:605); focused collapsed-caret animation schedules its next phase through [`caret_blink_visible()`](../../../../apps/scriptcraft/src/main.rs:973). The unconditional focused-idle request rejected by the prior validation is absent. The benchmark reports zero repaint requests for unchanged cached reads and one per sampled edit case, consistent with the documented policy. |
| Incremental pagination | **PASS / explicitly deferred** | No incremental pagination or page-frontier strategy was added. The evidence records complete-layout measurement without proof that downstream pagination dominates, matching the approved research boundary in [`02-layout-pagination-rendering.md`](../../../.analysis/stage2/02-layout-pagination-rendering.md:176). |
| Scope and rogue behavior | **PASS** | Product changes are limited to the shared core cache seam, app diagnostics/boundaries, and the focused responsiveness benchmark named in [`WO-006a-complete-responsiveness-measurement.md`](../work-orders/WO-006a-complete-responsiveness-measurement.md:53). The fix adds no renderer replacement, second text buffer, document-text logging, incremental pagination, backdoor, deceptive behavior, or unrelated editor feature. |
| Durable-document impact | **PASS WITH NOTE** | The fix correctly classifies requirements as `verify unchanged` and blueprint updates as measurement/cache-ownership clarification in [`WO-006a-complete-responsiveness-measurement.md`](../work-orders/WO-006a-complete-responsiveness-measurement.md:46). The core, desktop, and writing-loop blueprints record the shared cache, invalidation, privacy, and redraw boundary at [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:101), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:86), and [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:120). The writing-loop blueprint still contains the predecessor below-resolution benchmark wording at [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:124); this is a documentation freshness note, not a contradiction of the corrected implementation or a new product-code requirement. |

## Required Cargo gates and evidence boundary

The fix work order and execution log record passing results for [`cargo fmt --all -- --check`](../../../../Cargo.toml:1), [`cargo check --workspace`](../../../../Cargo.toml:1), focused package/benchmark tests, [`cargo test --workspace`](../../../../Cargo.toml:1), [`cargo clippy --workspace --all-targets -- -D warnings`](../../../../Cargo.toml:1), [`cargo build --workspace`](../../../../Cargo.toml:1), and [`git diff --check`](../../../../Cargo.toml:1) at [`WO-006a-complete-responsiveness-measurement.md`](../work-orders/WO-006a-complete-responsiveness-measurement.md:58) and [`execution-log.md`](../execution-log.md:109). The recorded focused results are 30 app tests and 2 responsiveness tests; the full gate sequence is recorded as passed.

These commands were not independently rerun in this repository-reading context because no command-execution tool was available. Under [`05-validation.md`](../../../process/05-validation.md:23), the specific, current, and internally consistent recorded results are accepted with a warning rather than treated as a blocker. [`project-manifest.md`](../../../project-manifest.md:88) configures no required native/manual hard-stop gate. Native long-document smoke remains unavailable and is not claimed as passed, consistent with [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:90).

### W-001 — Cargo gates are recorded, not independently rerun

- **Severity:** warning
- **Evidence:** The final fix gate sequence is recorded in [`WO-006a-complete-responsiveness-measurement.md`](../work-orders/WO-006a-complete-responsiveness-measurement.md:59) and [`execution-log.md`](../execution-log.md:109); this validation context could inspect but not execute commands.
- **Required next state:** Note only. Do not create another fix work order on this basis; retain the recorded gate evidence and rerun in a command-capable session if the orchestrator requires independent command confirmation.

### W-002 — Native long-document smoke unavailable

- **Severity:** warning
- **Evidence:** The fix evidence explicitly classifies native long-document smoke as unavailable at [`WO-006a-complete-responsiveness-measurement.md`](../work-orders/WO-006a-complete-responsiveness-measurement.md:59), and project policy has no configured native/manual hard-stop gate at [`project-manifest.md`](../../../project-manifest.md:88).
- **Required next state:** Note only. Do not infer native success and do not create a fix work order.

### W-003 — One durable benchmark sentence is stale

- **Severity:** note
- **Evidence:** [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:124) retains the original below-resolution cached wording, while the corrected fix evidence reports `0.001667 ms` actual-cache p95 at [`WO-006a-complete-responsiveness-measurement.md`](../work-orders/WO-006a-complete-responsiveness-measurement.md:58).
- **Required next state:** Documentation-only follow-up if the orchestrator wants all durable benchmark prose to match the corrected evidence; this is not a validation hard stop and no product code or existing artifact should be reopened for it within this validation.

## Final verdict and exact next state

- **Implementation alignment:** **PASS.** All three authorized hard stops from [`validation-WO-006-v2.md`](validation-WO-006-v2.md:23) are resolved: actual frame/event timing totals advance at their app boundaries, the benchmark calls the shared [`LayoutCache`](../../../../crates/scriptcraft-core/src/layout.rs:63) rather than measuring an `Arc` clone, and every approved case prints p50/p95 plus the required frame/event/layout/paint/repaint counters.
- **Preserved boundaries:** Cache invalidation, privacy, deterministic shared geometry and independent export layout, redraw policy, incremental-pagination deferral, scope, and durable-impact classification remain aligned. No rogue behavior was observed.
- **Warnings/notes:** Recorded Cargo gates were not independently rerun here; native long-document smoke is unavailable and optional; one blueprint sentence contains predecessor benchmark wording.
- **Fix work order required:** `no`.
- **Exact next state:** Mark [`WO-006a-complete-responsiveness-measurement.md`](../work-orders/WO-006a-complete-responsiveness-measurement.md:1) `complete`, then complete the original [`WO-006-long-document-responsiveness.md`](../work-orders/WO-006-long-document-responsiveness.md:1) through this passing fix-chain validation. Preserve the warnings/notes above, do not add incremental pagination, and do not claim native long-document smoke passed.

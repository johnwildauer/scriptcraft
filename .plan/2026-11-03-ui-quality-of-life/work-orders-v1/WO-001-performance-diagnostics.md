# RS-WO-001 — Privacy-safe performance diagnostics

- **Status:** Implemented 2026-11-03; user validation pending; not user-validated
- **Initiative:** [`ui-quality-of-life-v1-render-speed.md`](../ui-quality-of-life-v1-render-speed.md)
- **Depends on:** None; execute first
- **Validation owner:** User; no agent-run checks or independent validators

## Objective

Add resettable, privacy-safe phase timings and work counters so later orders can prove where work occurs without using wall-clock thresholds.

This order changes diagnostics only. It does not change rendering, editing, layout, refresh scheduling, persistence, or exports.

## Implementation steps

1. Extend [`PerformanceDiagnostics`](../../../apps/scriptcraft/src/main.rs:71) with resettable measurements for frame, event dispatch, layout lookup, element wrapping, page painting, caret lookup, and derived refresh.
2. Add counters for:
   - pages considered and pages painted;
   - lines and line/selection spans submitted for text layout;
   - galleys built and reused;
   - element wraps built and reused;
   - complete layouts built and layout object identity changes;
   - caret lookups and indexed versus linear interaction queries;
   - derived refreshes attempted, completed, skipped, changed, unchanged, and deferred.
3. Add an explicit per-benchmark reset operation. Keep cumulative counters where existing diagnostics rely on them, but make delta capture possible without losing prior state.
4. Project the new values through [`DiagnosticProjection`](../../../apps/scriptcraft/src/status_bar.rs:744) and its existing single-line diagnostics presentation.
5. Keep all output limited to numbers, phase names, build-profile labels, and operation labels. Never emit screenplay text, FDX content, paths, IDs, or source bytes.
6. Add a test-only instrumentation seam for later orders to assert cache reuse and reconciliation counts without timing assertions.

## Structural acceptance

- Diagnostics can distinguish a whole-layout cache hit from actual painting and from per-element wrap reuse.
- A reset changes diagnostic state only; it does not change session revision, selection, dirty state, history, layout identity, or output.
- Counters use saturating arithmetic and remain well-defined across document replacement and long-running sessions.
- No diagnostic string contains document-owned content.

## Regression coverage to author

Add focused desktop tests for counter increments, reset isolation, saturation boundaries, replacement behavior, and sanitized projection. Tests must assert exact counters rather than elapsed time.

## Durable documentation

On implementation, record the diagnostic ownership and privacy boundary in [`technical-context.md`](../../../.forge/blueprints/technical-context.md:43) and link the execution record back to this order and the initiative document.

## Implementation record — 2026-11-03

Implemented in [`main.rs`](../../../apps/scriptcraft/src/main.rs) and [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs), with value-free wrap measurements returned separately by [`layout.rs`](../../../crates/scriptcraft-core/src/layout.rs). The recorder retains saturating cumulative totals across document replacement; `reset_benchmark` snapshots those totals under a closed operation label, and `benchmark_delta` reports saturating deltas without mutating session or layout state. The status-bar summary contains only numeric measurements, fixed counter/phase names, a debug/release label, and a closed operation label. Core wrap timings isolate actual element-wrap calls and do not enter layout equality.

Counters cover complete-layout builds/cache hits, layout identity changes, pages considered/painted, lines and selected-line spans submitted, element wraps built/reused, galley builds/reuse, caret lookups, indexed/linear interaction queries, and derived refresh attempted/completed/skipped/changed/unchanged/deferred outcomes. At the RS-WO-001 handoff, per-element wrap reuse, explicit galley build/reuse, and indexed interaction queries were zero by design; RS-WO-005 now increments galley counters, RS-WO-006 increments wrap reuse, RS-WO-007 increments indexed interaction counts, and RS-WO-008 distinguishes refresh gate skips and successful changed/unchanged/deferred outcomes. Existing whole-layout cache hits are reported separately. The test-only `snapshot_for_test` accessor is the instrumentation seam for later work orders.

Authored desktop regression cases are `performance_diagnostic_counters_increment_and_saturate`, `performance_benchmark_reset_is_diagnostic_only_and_captures_deltas`, `performance_diagnostics_survive_document_replacement_without_source_data`, `derived_refresh_diagnostics_separate_idle_skip_from_attempt_outcome`, and `performance_projection_contains_only_safe_counters_and_labels`; the existing pointer-hit test also now asserts exact linear/indexed query counts. They cover layout build/cache-hit and wrap counts, saturating counter/time addition, reset isolation and deltas, refresh outcome counts, document replacement continuity, pointer-query classification, and sanitized projection. These tests were authored but not run.

### WO-002 handoff and limitations

WO-002 consumed the desktop counters through `snapshot_for_test` and the diagnostic summary and authored real egui surface assertions for page, line, selection, scroll, input, zoom, caret, and refresh work. At that handoff, galley reuse and indexed-query expectations were zero until WO-005 and WO-007 respectively; per-element wrap reuse was zero until WO-006. The WO-005 handoff below supersedes only the galley zero expectation. WO-002 implementation does not optimize rendering or editing. No timing threshold is defined. User review and validation remain pending; no review, validation, build, test, lint, formatting, or manual verification was performed by the agent.

The benchmark reset is an internal recorder/test-seam operation; this diagnostics-only order adds no user-facing reset control. WO-002's same-module desktop tests can set a closed operation label and capture exact deltas without changing session state.

## User validation

Inspect the baseline diagnostics in debug and release builds. Confirm counters advance for the expected operation, reset cleanly, expose no document content, and do not alter ordinary editing or display behavior.

## Handoff

RS-WO-002 consumes these counters in a real desktop render-path oracle. No optimization is authorized by this order.

### RS-WO-005 handoff — 2026-11-03

RS-WO-005 now increments `galleys-built` and `galleys-reused` per visible nonempty line/color span, with span cache keys private to the egui context and no text values in diagnostics. The desktop oracle's old zero-galley baseline is retired. The counters remain separate from per-element wrap reuse and indexed-interaction counts. WO-001–WO-005 coverage remains unexecuted and user review/validation remains pending.

### RS-WO-006 handoff — 2026-11-03

RS-WO-006 now increments the existing value-free `element-wraps-built` and `element-wraps-reused` totals for actual cache misses/hits during a complete layout build. An unchanged complete-layout cache hit reports neither, because no element wrap lookup/build pass runs. Wrap time remains attributed only to actual `wrap_element` calls. This is separate from WO-005 galley counts and whole-layout hit/build counts. Coverage is authored but remains unexecuted; user review and validation remain pending.

### RS-WO-008 handoff — 2026-11-03

Derived-refresh diagnostics now report core refresh calls as attempts, successful returns as completions, same-revision/not-yet-due polls as skips, successful changed and unchanged results separately, and safe-stop/error deferrals independently. The added values are numeric only; no revision, document identity, content, or source data is projected. Authored expectations remain unrun and user validation is pending.

### RS-WO-009 handoff — 2026-11-03

Derived-refresh diagnostics now also report the number of detached top-level project candidates, managed-heading scene candidates, and redundant complete preservation-state clones performed during successful scheduled refreshes. These are saturating numeric counters only. The intended changed-revision structural profile is one project candidate, one scene candidate per heading considered, and zero additional whole-preservation clones per heading; the top-level candidate's required preservation clone is represented by the project-candidate count, not double-counted as redundant. Error attempts have no correction outcome and therefore do not add candidate counts. Coverage is authored but unrun; no performance or validation result is claimed.

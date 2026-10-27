# Work Order: WO-006a — Complete Responsiveness Measurement

## Metadata

- **Status:** `complete`
- **Assignee:** Fresh Code-mode implementer after WO-006 validation failure
- **Phase/order:** `fix for 6 of 6`
- **Initiative:** [`../editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md)
- **Original work order:** [`WO-006-long-document-responsiveness.md`](WO-006-long-document-responsiveness.md)
- **Failing validation:** [`../validations/validation-WO-006-v2.md`](../validations/validation-WO-006-v2.md)
- **Validation:** [`../validations/validation-WO-006a-v2.md`](../validations/validation-WO-006a-v2.md)

## Summary

Complete the WO-006 measurement contract so benchmark output measures the implemented desktop cache path and reports all required bounded counters.

## In scope

- Populate frame, event, layout-build, paint, and repaint-request diagnostics at their actual app boundaries.
- Change the benchmark to exercise the same revision/profile-keyed cache path used by the desktop app rather than measuring only an `Arc` clone.
- Report per-case warmup/sample timing and frame/layout/paint/repaint counters for the approved page/edit matrix.
- Preserve the existing cache keys, invalidation, redraw policy, privacy boundary, and incremental-pagination deferral.
- Add focused assertions proving the benchmark observes a cache miss/build followed by a cache hit.

## Out of scope

- New caching strategy, incremental pagination, renderer replacement, document text logging, or changes to unrelated editor behavior.

## Requirements and blueprints

- **Requirements (verbatim):** `NFR-WRITING-002`: “Core editing, layout, and persistence behavior shall be exercisable without a desktop window for deterministic tests.” `NFR-WRITING-003`: “Native serialization and structural pagination outputs shall be deterministic for the same semantic project/profile inputs.”
- **Blueprints:** [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md).

## Implementation guidance

Expose or reuse a bounded cache/diagnostic seam so the integration benchmark invokes the same cache lookup/build behavior as [`live_document_surface()`](../../../apps/scriptcraft/src/main.rs:409). Keep the seam UI-independent where practical. Counters must be numeric/phase labels only and must not retain document text. Report p50/p95 and all five counters per case.

## Validation

- Run all Cargo gates.
- Confirm the benchmark uses the implemented cache path, not an `Arc` clone proxy.
- Verify 3 warmups and 10 samples for 1/5/15/50/120/180-page fixtures and beginning/middle/end edits.
- Verify unchanged-read cache hit, revision/profile invalidation, deterministic caret/hit/PDF behavior, and incremental pagination remains explicitly deferred.
- Native long-document smoke remains optional/unavailable under project policy.

## Durable-document impact

- **Requirements:** `verify unchanged` — this corrects evidence/instrumentation for the approved responsiveness contract.
- **Blueprint:** `update` only to clarify actual instrumentation/cache ownership evidence.
- **Rationale/evidence:** [`validation-WO-006-v2.md`](../validations/validation-WO-006-v2.md) found declared-but-unpopulated counters, an `Arc`-clone benchmark proxy, and missing per-case counter output.
- **Authorized documentation changes:** Update the original WO, this fix WO, affected blueprints, and [`execution-log.md`](../execution-log.md) with measurement evidence.

## Implementation evidence — 2026-10-26

- **Changed product files:** [`crates/scriptcraft-core/src/layout.rs`](../../../../crates/scriptcraft-core/src/layout.rs), [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs), and [`crates/scriptcraft-core/tests/wo006_responsiveness.rs`](../../../../crates/scriptcraft-core/tests/wo006_responsiveness.rs).
- The reusable core [`LayoutCache`](../../../../crates/scriptcraft-core/src/layout.rs:69) now owns the revision/complete-`PageProfile` lookup/build seam used by both the desktop adapter and the benchmark. The benchmark no longer measures an `Arc` clone; its unchanged 180-page case performs the real cache lookup and reports stable microsecond-scale timings.
- Frame and event timing totals are accumulated at the actual [`eframe::App::update()`](../../../../apps/scriptcraft/src/main.rs:1052) and [`dispatch_surface_events()`](../../../../apps/scriptcraft/src/main.rs:646) boundaries. Layout-build timing remains around cache misses, page-paint timing around each [`paint_page()`](../../../../apps/scriptcraft/src/main.rs:404), and repaint requests at [`request_repaint()`](../../../../apps/scriptcraft/src/main.rs:101). A focused regression asserts event timing advances; existing rendered tests cover the paint path. Diagnostics remain aggregate numeric data only.
- The controlled benchmark retains exactly 3 warmups and 10 samples for 1/5/15/50/120/180 pages and beginning/middle/end edits. Every case prints p50/p95 milliseconds and frames/events/layout-builds/cache-hits/painted-pages/repaint-request counters. The recorded run reported 180-page unchanged uncached p95 `19.981709 ms` versus actual-cache p95 `0.001667 ms`, a `100.0%` reduction; cached samples recorded 10 frames, 0 layout builds, 10 cache hits, 1800 painted pages, and 0 repaint requests. Edit cases recorded 10 frames, 10 events, 10 layout builds, 10 repaint requests, and page-count-scaled paint counts.
- Focused and full Cargo gates passed after the fix. Incremental pagination remains explicitly deferred; cache keys, invalidation, privacy, redraw policy, deterministic caret/hit/PDF behavior, and the native-smoke-unavailable boundary are unchanged.

## Validation outcome

- Fresh validation [`validation-WO-006a-v2.md`](../validations/validation-WO-006a-v2.md) returned `PASS WITH WARNINGS`.
- All three hard-stop corrections were confirmed; warnings cover recorded-but-not-independent gate reruns and unavailable native smoke.
- State transition: `in-review → complete`; no additional fix work order is required.

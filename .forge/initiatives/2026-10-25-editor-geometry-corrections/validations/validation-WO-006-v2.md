# Validation — WO-006 Long-Document Responsiveness (v2)

## Validation metadata

- **Validation status:** `FAIL`
- **Validated work order:** [`WO-006-long-document-responsiveness.md`](../work-orders/WO-006-long-document-responsiveness.md:1)
- **Initiative:** [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:1)
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `architect-fresh-context-editor-geometry-corrections-v2-WO-006-validation-2026-10-26`
- **Session exception:** `none`
- **Date:** `2026-10-26`
- **Product-code changes:** none
- **Existing-artifact changes:** none; this file is the new validation artifact

## Validation boundary

This fresh validation assesses only the bounded privacy-safe instrumentation, benchmark protocol and results, approved cache threshold, revision/profile invalidation, shared geometry use for caret/hit/navigation and deterministic PDF behavior, redraw policy, incremental-pagination deferral, scope and durable-document impact, and the required Cargo gates. It does not assess unrelated v2 work orders, general UI quality, native visual behavior beyond the stated evidence boundary, or future pagination architecture.

The comparison basis was [`05-validation.md`](../../../process/05-validation.md:1), the approved v2 performance oracle in [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:73), the reconciled design review in [`design-review-v2.md`](../reviews/design-review-v2.md:146), the approved work order and its implementation evidence, the v2 execution log, the WoodCraft-derived research boundary, the applicable requirements, the current source, the focused responsiveness test, the updated blueprints, and the available changed-file/evidence record. The evidence is sufficient for meaningful comparison; no standalone branch diff was available in this inspection context.

## Findings

### EGC-V2-WO006-F-001 — Required frame and event timing instrumentation is not implemented

- **Severity:** hard stop; required criterion unmet.
- **Evidence:** The work order requires bounded frame, event-dispatch, layout-build, page-paint, and repaint-request counters/timings at [`WO-006-long-document-responsiveness.md`](../work-orders/WO-006-long-document-responsiveness.md:17). [`PerformanceDiagnostics`](../../../../apps/scriptcraft/src/main.rs:34) declares `frame_nanos` and `event_nanos`, but the inspected frame path only increments `frames` at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1053), and the event path increments `events` and creates an `Instant` at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:655) without adding elapsed time to `event_nanos`. No inspected path adds elapsed time to `frame_nanos`. The status surface reports counts only at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1118-1127).
- **Impact:** The implementation has fields that suggest frame/event timings, but it does not produce the required frame/event timing evidence. This prevents validating the approved instrumentation boundary rather than merely identifying a style or maintainability opportunity.
- **Required correction:** Record bounded elapsed frame and event-dispatch durations in the corresponding diagnostic totals, and expose/report the resulting aggregate timing evidence without retaining document text. Add focused regression or instrumentation evidence that the totals advance on the relevant paths.
- **Required next state:** Keep [`WO-006-long-document-responsiveness.md`](../work-orders/WO-006-long-document-responsiveness.md:1) `in-review`; create a letter-suffixed fix work order within the authorized WO-006 boundary, dispatch a new implementer session, rerun the required gates, and obtain a separate fresh validation.

### EGC-V2-WO006-F-002 — The benchmark's cached comparison does not measure the implemented cache path

- **Severity:** hard stop; the approved cache threshold is not demonstrated by the submitted benchmark.
- **Evidence:** The approved oracle requires an unchanged-read cache hit and at least a 30% p95 reduction in the measured bottleneck at [`WO-006-long-document-responsiveness.md`](../work-orders/WO-006-long-document-responsiveness.md:51). The implemented desktop cache is exercised by [`layout_for_render()`](../../../../apps/scriptcraft/src/main.rs:114), and the focused app regression verifies one unchanged hit plus revision/profile misses at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1222-1243). However, the benchmark's `measure_unchanged_cache()` only clones an already-built [`Arc<ScreenplayLayout>`](../../../../crates/scriptcraft-core/tests/wo006_responsiveness.rs:67-79); it does not call [`layout_for_render()`](../../../../apps/scriptcraft/src/main.rs:114), reproduce the [`LayoutCache`](../../../../apps/scriptcraft/src/main.rs:48) key check, or measure the desktop cache adapter. The reported `18.998 ms` versus below-resolution `0.001 ms` result therefore compares a complete core build with an `Arc` clone, not an uncached versus implemented-cache render/session path.
- **Impact:** The unchanged-read hit is source/test evidenced, but the claimed 100.0% cache p95 reduction is not a valid before/after measurement of the shipped cache. The approved `>=30%` threshold cannot be accepted from this benchmark alone. The below-resolution cached value also prevents a stable quantitative p95 comparison at the reported precision.
- **Required correction:** Replace or supplement the benchmark with a privacy-safe harness that exercises the actual revision/profile-keyed cache adapter for the unchanged 180-page read, reports the same warmup/sample protocol and sufficient timer precision, and demonstrates the approved threshold. Retain separate edit cases and report the required counters alongside p50/p95 values.
- **Required next state:** Resolve this finding in a new WO-006 fix chain and obtain fresh validation; do not promote the cache threshold claim to a completed acceptance result before then.

### EGC-V2-WO006-F-003 — Benchmark output omits the required per-case counters

- **Severity:** hard stop; benchmark evidence is incomplete against the approved oracle.
- **Evidence:** The approved protocol requires p50/p95 milliseconds plus frame, layout-build, paint, and repaint-request counts at the target page sizes and edit positions at [`WO-006-long-document-responsiveness.md`](../work-orders/WO-006-long-document-responsiveness.md:51). The focused benchmark prints only `pages`, edit position, resulting page count, p50, and p95 at [`wo006_responsiveness.rs`](../../../../crates/scriptcraft-core/tests/wo006_responsiveness.rs:82-105). It contains no frame, page-paint, repaint-request, or event-dispatch counter collection. The desktop diagnostics declaration exists at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:34-45), but those counters are not connected to the core benchmark output.
- **Impact:** The timing table cannot establish whether the observed cost is accompanied by repeated frame/paint/repaint work, nor can it verify the complete requested benchmark protocol. The missing output is material because the work order specifically makes redraw policy and instrumentation part of the acceptance boundary.
- **Required correction:** Extend the controlled rendered/headless evidence to report the required counters for every target page/edit case, while keeping diagnostics aggregate and text-free. Re-record the benchmark evidence and update only the authorized WO/log documentation.
- **Required next state:** Fix within WO-006's approved boundary, then revalidate independently.

## Passing implementation-alignment checks

| Boundary | Result | Evidence and assessment |
|---|---|---|
| Privacy-safe bounded diagnostics | **PASS** | [`PerformanceDiagnostics`](../../../../apps/scriptcraft/src/main.rs:34-45) contains aggregate numeric counters/totals only, and the implementation evidence states that document text is not retained at [`WO-006-long-document-responsiveness.md`](../work-orders/WO-006-long-document-responsiveness.md:61). The status surface exposes aggregate values at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1118-1127). The missing frame/event timing accumulation is recorded separately as a hard stop. |
| Cache ownership and key shape | **PASS with evidence limitation** | [`LayoutCache`](../../../../apps/scriptcraft/src/main.rs:47-52) is desktop-session-owned and [`layout_for_render()`](../../../../apps/scriptcraft/src/main.rs:114-144) checks session revision plus complete `PageProfile` equality before returning the immutable [`Arc<ScreenplayLayout>`](../../../../apps/scriptcraft/src/main.rs:124-143). The updated core blueprint records the profile dimensions covered by the equality boundary at [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:103-105). The focused app test proves unchanged-read hit and revision/profile misses at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1222-1243). The benchmark does not validly measure this path; see F-002. |
| Revision/profile invalidation | **PASS** | The cache key compares the editing revision and complete persisted profile at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:117-121). The focused regression changes `line_advance` and then performs a session edit, requiring layout builds 2 and 3 at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1230-1242). Viewport scale is used only in painting and is not part of the cache key at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:420-466), matching the approved presentation-only rule. |
| Shared caret/hit/navigation geometry | **PASS by source alignment** | The live surface paints from the cached layout at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:522-547), pointer placement consumes the same layout at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:604-611), caret geometry uses it at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:470-503), and vertical navigation obtains it through [`layout_for_render()`](../../../../apps/scriptcraft/src/main.rs:844-849). Existing deterministic core/layout and app regressions remain the relevant evidence boundary. |
| PDF determinism boundary | **PASS by inspected design/evidence** | The updated core blueprint explicitly records that PDF requests a fresh core layout independent of transient desktop cache state at [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:103-105). This preserves deterministic export rather than coupling PDF output to viewport/cache state. The work order records existing layout/caret/hit/PDF tests as the determinism boundary at [`WO-006-long-document-responsiveness.md`](../work-orders/WO-006-long-document-responsiveness.md:75). |
| Redraw policy | **PASS with instrumentation limitation** | Active input and scroll request repaint at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:759-776); focus transitions request a transition repaint at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:625-635); focused collapsed-caret animation schedules the next phase at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:993-1015); and no unconditional focused-idle request appears in [`handle_surface_input()`](../../../../apps/scriptcraft/src/main.rs:759-776). The focused-caret path increments the repaint diagnostic when focused at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:497-500). Required complete counter/timing reporting remains unmet under F-001/F-003. |
| Incremental pagination decision | **PASS** | The implementation explicitly defers incremental pagination because the recorded data measures complete layout cost and does not prove downstream pagination dominance at [`WO-006-long-document-responsiveness.md`](../work-orders/WO-006-long-document-responsiveness.md:74-75). This is aligned with the WoodCraft research boundary, which recommends session/paragraph caching first and a downstream-dominance benchmark before incremental pagination at [`02-layout-pagination-rendering.md`](../../../.analysis/stage2/02-layout-pagination-rendering.md:176-190). |
| Scope and rogue behavior | **PASS** | The changed product record is limited to [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1), the focused benchmark [`wo006_responsiveness.rs`](../../../../crates/scriptcraft-core/tests/wo006_responsiveness.rs:1), and the three authorized blueprint documents at [`WO-006-long-document-responsiveness.md`](../work-orders/WO-006-long-document-responsiveness.md:57-62). No renderer replacement, second buffer, GPU architecture, premature pagination, deceptive behavior, backdoor, malicious behavior, or unrelated product area was observed. |
| Durable-document impact | **PASS** | The work order classifies requirements as `verify unchanged` and the blueprint as `update` for measured cache ownership/invalidation/redraw facts at [`WO-006-long-document-responsiveness.md`](../work-orders/WO-006-long-document-responsiveness.md:43-46). The three updated blueprints record only those measured responsiveness facts at [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:101-105), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:86-90), and [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:120-124). |

## Required Cargo gates

The implementation handoff records passing results for [`cargo fmt --all -- --check`](../../../../Cargo.toml:1), [`cargo check --workspace`](../../../../Cargo.toml:1), [`cargo test --workspace`](../../../../Cargo.toml:1), [`cargo clippy --workspace --all-targets -- -D warnings`](../../../../Cargo.toml:1), [`cargo build --workspace`](../../../../Cargo.toml:1), and [`git diff --check`](../../../../Cargo.toml:1) at [`WO-006-long-document-responsiveness.md`](../work-orders/WO-006-long-document-responsiveness.md:76) and [`execution-log.md`](../execution-log.md:99). The recorded workspace test summary states 30 app tests, 44 core unit tests, 2 paste-safety tests, 5 long-document tests, 4 casing tests, 6 parenthetical tests, 1 responsiveness benchmark, 3 E2E tests, and 0 doc-test failures.

These gates were not independently rerun in this repository-reading validation context. Per the user's direction and [`05-validation.md`](../../../process/05-validation.md:23-27), the specific and internally consistent recorded results are accepted as evidence with a warning, not treated as a blocker. No project-policy-required native/manual hard-stop gate is configured; native long-document smoke remains unavailable and is not claimed as passed.

## Warnings and notes

### W-001 — Cargo gates are recorded, not independently rerun

- **Severity:** warning.
- **Evidence:** The passing gate sequence is recorded in [`WO-006-long-document-responsiveness.md`](../work-orders/WO-006-long-document-responsiveness.md:76) and [`execution-log.md`](../execution-log.md:99), but this validation context did not execute commands.
- **Required next state:** Note only. Rerun the full sequence after the required fix implementation.

### W-002 — Native long-document smoke is unavailable

- **Severity:** warning.
- **Evidence:** The implementation evidence explicitly says native long-document smoke was unavailable and not claimed at [`WO-006-long-document-responsiveness.md`](../work-orders/WO-006-long-document-responsiveness.md:75,81). The plan and work order classify headless/rendered evidence as required and native smoke as optional/unavailable at [`WO-006-long-document-responsiveness.md`](../work-orders/WO-006-long-document-responsiveness.md:55).
- **Required next state:** Note only; do not create a fix work order or infer native success from the rendered/headless evidence.

### W-003 — No standalone branch diff was available

- **Severity:** warning.
- **Evidence:** The comparison used the changed-file record, execution log, current source/tests, and updated blueprints. No standalone comparable Git diff was available to this validation session.
- **Required next state:** Note only. The available evidence was sufficient for meaningful comparison, but a future fix validation should attach the comparable diff if available.

## Final verdict and exact next state

- **Implementation alignment:** **Fail.** The cache ownership/key and redraw design are within scope, privacy-safe, and substantially implemented. However, the required frame/event timing instrumentation is not populated, and the submitted benchmark does not measure the implemented cache path or report the required per-case counters. These are concrete unmet WO-006 acceptance criteria, not optional quality suggestions.
- **Hard-stop findings:** [`EGC-V2-WO006-F-001`](#egc-v2-wo006-f-001--required-frame-and-event-timing-instrumentation-is-not-implemented), [`EGC-V2-WO006-F-002`](#egc-v2-wo006-f-002--the-benchmarks-cached-comparison-does-not-measure-the-implemented-cache-path), and [`EGC-V2-WO006-F-003`](#egc-v2-wo006-f-003--benchmark-output-omits-the-required-per-case-counters).
- **Fix work order required:** `yes`.
- **Exact next state:** Keep [`WO-006-long-document-responsiveness.md`](../work-orders/WO-006-long-document-responsiveness.md:1) `in-review`. Create a letter-suffixed WO-006 fix work order limited to completing frame/event timing accumulation and the privacy-safe benchmark evidence: exercise the actual revision/profile cache adapter, report frame/layout/paint/repaint-request counters with p50/p95 data for all approved page/edit cases, and preserve deterministic caret/hit/PDF and redraw tests. Dispatch a new implementer session, rerun all required Cargo gates, then obtain a separate fresh validation artifact. Do not mark WO-006 `complete`, do not treat the cache threshold as proven from the current benchmark, and do not add incremental pagination.

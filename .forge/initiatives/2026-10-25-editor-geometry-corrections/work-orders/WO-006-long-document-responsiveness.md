# Work Order: WO-006 — Long-Document Responsiveness

## Metadata

- **Status:** `complete`
- **Assignee:** Fresh Code-mode implementer after WO-005 validation
- **Phase/order:** `6 of 6`
- **Initiative:** [`../editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md)
- **Validation:** [`../validations/validation-WO-006.md`](../validations/validation-WO-006.md)
- **Fixes:** [`WO-006a-complete-responsiveness-measurement.md`](WO-006a-complete-responsiveness-measurement.md)

## Summary

Investigate frame/layout cost on long documents and implement the smallest measured caching or redraw correction that makes the editor responsive without weakening geometry correctness.

## In scope

- Instrument bounded frame, event-dispatch, layout-build, page-paint, and repaint-request counters/timings without retaining document text.
- Benchmark controlled documents at 1, 5, 15, 50, and larger page counts, with edits near the beginning, middle, and end.
- Identify repeated layout/paint work and add revision/profile-keyed layout caching if measurements justify it.
- Repaint only for active input, focused caret animation, or explicit state changes; preserve focus and blink behavior.
- Record whether incremental pagination is justified; if not, explicitly defer it.

## Out of scope

- Premature page-frontier/incremental pagination, renderer/backend replacement, GPU texture architecture, or broad profiling infrastructure.

## Research constraints

Use the documented WoodCraft techniques in [`02-layout-pagination-rendering.md`](../../../.analysis/stage2/02-layout-pagination-rendering.md:157): paragraph cache keys include metric inputs, session layout is revision-keyed, screen/PDF share geometry, and incremental pagination is added only after a long-script benchmark demonstrates downstream pagination is dominant. The current app's unconditional [`handle_surface_input()`](../../../apps/scriptcraft/src/main.rs:633) repaint path and complete [`ScreenplayLayout::build()`](../../../apps/scriptcraft/src/main.rs:410) rebuild are primary hypotheses, not assumed defects.

## Requirements and blueprints

- **Requirements:** `NFR-WRITING-002`, `NFR-WRITING-003` in [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md).
- **Blueprints:** [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md).

## Validation

- Record before/after measurements and fixture dimensions.
- Test cache hits for unchanged revision/profile, misses after edits/profile changes, caret/hit correctness, and save/export determinism.
- Confirm focused caret blink remains smooth and unfocused idle frames do not spin unnecessarily.
- Run all Cargo gates and record native limitations.

## Durable-document impact

- **Requirements:** `verify unchanged` unless a measurable responsiveness threshold is promoted to a requirement.
- **Blueprint:** `update` with cache ownership, invalidation keys, and any confirmed redraw policy.

## Reconciled handoff

- **Applicable requirements (verbatim):** `NFR-WRITING-002`: “Core editing, layout, and persistence behavior shall be exercisable without a desktop window for deterministic tests.” `NFR-WRITING-003`: “Native serialization and structural pagination outputs shall be deterministic for the same semantic project/profile inputs.”
- **Benchmark oracle:** Warm up three runs, collect ten samples, and report p50/p95 milliseconds plus frame/layout/paint/repaint-request counts at 1/5/15/50/120/180 pages with edits near beginning/middle/end. A cache must demonstrate an unchanged-read hit and >=30% p95 reduction in the measured bottleneck without >10% p95 regression elsewhere; otherwise record a no-cache decision. Incremental pagination requires evidence that downstream pagination dominates.
- **Cache contract:** Any cache key must include session revision and every geometry/profile input that affects layout, including paper/margins, role boxes, line advance, boundary policy, font family/size/metric, and presentation mode where applicable. Cache invalidation and deterministic export tests are mandatory.
- **Rationale/evidence:** Research recommends revision-keyed layout and paragraph caches before incremental pagination; current [`live_document_surface()`](../../../apps/scriptcraft/src/main.rs:409) rebuilds complete layout and [`handle_surface_input()`](../../../apps/scriptcraft/src/main.rs:633) requests repaint, making both measurable hypotheses.
- **Authorized documentation changes:** Update [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md), and [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md) only for measured cache ownership/invalidation facts; record benchmark data in this WO and the execution log.
- **Native evidence boundary:** Headless benchmark and rendered instrumentation are required; native long-document smoke is optional, and unavailable native data is recorded as unavailable rather than passed.

## Validation outcome

- Initial validation [`validation-WO-006-v2.md`](../validations/validation-WO-006-v2.md) found three measurement-evidence hard stops.
- Fix [`WO-006a-complete-responsiveness-measurement.md`](WO-006a-complete-responsiveness-measurement.md) passed fresh validation [`validation-WO-006a-v2.md`](../validations/validation-WO-006a-v2.md) with warnings.
- State transition: `in-review → complete` through the WO-006a fix chain; incremental pagination remains deferred and no further fix is required.

## Implementation evidence — 2026-10-26

- **Changed product files:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs), [`crates/scriptcraft-core/tests/wo006_responsiveness.rs`](../../../../crates/scriptcraft-core/tests/wo006_responsiveness.rs).
- **Changed durable files:** [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md), and [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md).
- The desktop now records bounded aggregate frame, event, layout-build, page-paint, repaint-request counts and nanosecond totals. No diagnostic retains document text. Unconditional focused-idle repaint was removed; repaint requests remain tied to active events/scroll, focus transitions, and the existing focused caret phase.
- The desktop now shares an immutable layout through a revision/profile-keyed cache across rendering, hit/caret use, and vertical navigation. Cache misses are forced by any session revision change or any complete `PageProfile` change; viewport scale remains presentation-only. A focused app regression proves unchanged-read hits and revision/profile misses.
- The controlled benchmark fixture uses exactly three warmups and ten samples at 1/5/15/50/120/180 pages and edits near beginning/middle/end. The recorded run reported:

| Pages | Beginning p50/p95 ms | Middle p50/p95 ms | End p50/p95 ms |
|---:|---:|---:|---:|
| 1 | 0.242 / 0.249 | 0.251 / 0.317 | 0.250 / 0.254 |
| 5 | 0.983 / 1.087 | 0.791 / 0.872 | 0.724 / 0.733 |
| 15 | 1.849 / 1.916 | 1.618 / 1.673 | 1.549 / 1.567 |
| 50 | 5.178 / 5.226 | 5.246 / 6.613 | 5.183 / 5.263 |
| 120 | 12.498 / 12.658 | 12.530 / 12.668 | 12.488 / 12.648 |
| 180 | 18.823 / 19.098 | 18.732 / 18.879 | 18.786 / 18.959 |

- **Decision:** Implement the cache. The unchanged 180-page comparison measured 18.998 ms uncached p95 versus 0.000 ms displayed cached p95 (below the benchmark timer's 0.001 ms display resolution), a measured 100.0% layout-build p95 reduction, exceeding the approved >=30% threshold. No >10% p95 regression was observed in the edit cases. Incremental pagination is explicitly deferred: the data measures complete layout cost and does not prove downstream pagination dominant.
- **Determinism evidence:** The core fixture confirms page counts remain exactly 1/5/15/50/120/180 after edits; the app cache regression confirms profile/revision invalidation; existing layout/caret/hit/PDF tests remain the shared determinism boundary. Native long-document smoke was unavailable and is not claimed.
- **Cargo gates:** `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace` (30 app tests, 44 core unit tests, 2 paste-safety, 5 long-document, 4 casing, 6 parenthetical, 1 responsiveness benchmark, 3 E2E, 0 doc-test failures), `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build --workspace`, and `git diff --check` all passed.

## Follow-ups

- Fresh validation must independently inspect the cache key, repaint policy, benchmark output, all Cargo gates, and determinism evidence before any completion transition.
- Native long-document smoke remains optional/unavailable under the project policy and should be repeated only in a capable desktop session.

## WO-006a fix-chain evidence — 2026-10-26

- The three v2 hard stops are addressed by [`WO-006a-complete-responsiveness-measurement.md`](WO-006a-complete-responsiveness-measurement.md): frame/event timing is accumulated at the actual app boundaries, the benchmark invokes the shared core [`LayoutCache`](../../../../crates/scriptcraft-core/src/layout.rs:69), and every approved case reports p50/p95 plus all five required counters (with event and cache-hit detail also included).
- The corrected benchmark's unchanged 180-page comparison measured uncached p95 `19.981709 ms` versus actual-cache p95 `0.001667 ms` (`100.0%` reduction). The focused cache regression proves build then unchanged hit. Cache keys, invalidation, redraw policy, privacy, deterministic geometry/export boundaries, and incremental-pagination deferral were preserved.
- WO-006 remains `in-review`; WO-006a is `in-review` pending the separate fresh validation artifact [`validation-WO-006a-v2.md`](../validations/validation-WO-006a-v2.md).

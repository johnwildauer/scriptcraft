# V3-WO-001 — Repeated imported-scene derived-refresh oracle

- **Status:** Authored; user-owned validation pending (not run)
- **Initiative:** [`ui-quality-of-life-v3-derived-refresh-and-scene-heading.md`](../ui-quality-of-life-v3-derived-refresh-and-scene-heading.md)
- **Depends on:** None
- **Validation owner:** User; no agent-run checks, reviews, or validation

## Objective

Establish where a second or later imported-scene Character edit stops producing the expected affected-scene Cast List and scene-arc update, without changing the refresh algorithm.

## Scope

1. Add one focused core regression in [`v3_wo001_repeated_derived_refresh.rs`](../../../crates/scriptcraft-core/tests/v3_wo001_repeated_derived_refresh.rs) using an imported supported Scene Heading, Character cues, and a plain multi-run Cast List.
2. Complete the initial session refresh, then perform at least two successive Character rename/remove/add operations with an explicit refresh after each. Assert the affected-scene pending summary before each refresh and the resulting Cast List text/runs, retained Text runs, scene-arc names, and pending/retryable/deferred/failed summary afterward.
3. Include the post-first-update run shape explicitly: replacement text in the first run and empty later plain runs. Distinguish Complete activation scope from later affected-scene scopes.
4. Extend the existing desktop scheduler oracle only as needed to drive the same imported sequence through [`refresh_cast_lists_on_schedule()`](../../../apps/scriptcraft/src/main.rs:553), recording requested, attempted, changed, unchanged, deferred, and failed outcomes after every step.
5. Project the existing value-free changed-scene and unchanged-scene counts from [`CastListCorrection`](../../../crates/scriptcraft-core/src/cast_list.rs:22) into [`PerformanceDiagnostics::safe_summary()`](../../../apps/scriptcraft/src/main.rs:316). Do not expose scene IDs, names, text, paths, or source values.
6. Record the first failing stage and exact operation sequence in this order. If the sequence passes under user-owned execution, record that the original fixture/operation sequence is still required rather than claiming the report is resolved.

## Boundaries

- Do not change invalidation classification, retry suspension, Cast List correction, scene-arc reconciliation, save preparation, or scheduling cadence.
- Do not add broad logging or document-derived diagnostics.
- Do not weaken safe-stop deferral, fatal errors, or transactional rollback.

## Structural acceptance

- The authored sequence distinguishes no new invalidation, nonretryable pending work, an attempted unchanged pass, a deferred/failed pass, and a successful changed pass.
- A second and third refresh are exercised after real completion of the preceding scope, not through a fresh session's Complete invalidation.
- All new diagnostic values are numeric or closed categories only.

## Handoff

V3-WO-002 may repair only the failing branch established here or by equivalent user-supplied evidence. If no branch is established, V3-WO-002 remains blocked.

## Execution record — 2026-11-04

- **Authorization:** The user authorized execution of V3-WO-001 only. No V3-WO-002 repair or algorithm change was made.
- **Exact files and interfaces changed:**
  - [`crates/scriptcraft-core/tests/v3_wo001_repeated_derived_refresh.rs`](../../../crates/scriptcraft-core/tests/v3_wo001_repeated_derived_refresh.rs) adds an imported single-scene regression that completes initial Complete refresh, then performs a case-only rename, a distinct rename, text removal, and text addition, explicitly refreshing and asserting affected-scope state after each edit. It checks Cast List semantic text/runs, retained two-run shape, generated scene-arc names through serialized FDX, correction outcome scene counts, and pending/retryable/deferred/failed summaries.
  - [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs) adds numeric changed/unchanged scene totals to `PerformanceMeasurements`, their saturating delta projection, the `PerformanceDiagnostics::safe_summary()` output, and accumulation from each existing `CastListCorrection`. The desktop oracle drives the same imported sequence through the scheduler and checks per-step requested/attempted/changed/unchanged/deferred/failed totals. Existing deferred and failed scheduler oracles now assert the corresponding scene-count disposition. No public core API or refresh/scheduling algorithm changed.
  - Durable execution state was updated in this work order and [`ui-quality-of-life-v3-derived-refresh-and-scene-heading.md`](../ui-quality-of-life-v3-derived-refresh-and-scene-heading.md).
- **Authored but unrun coverage:** The new core regression, the imported desktop scheduler oracle, the existing safe-summary projection assertion extended for the two new numeric counters, and additional deferred/failed scheduler counter assertions. No build, test, lint, formatting, implementation check, review, or independent validation was run, in accordance with [`project-manifest.md`](../../../.forge/project-manifest.md).
- **Failing-stage disposition:** No failing stage for the repeated-refresh report is statically established. User-owned execution evidence is missing; therefore the first failing stage and exact user-fixture sequence remain unknown. The editor surfaced type-mismatch diagnostics in unchanged [`v3_wo0012_app_owned_scene_arcs.rs`](../../../crates/scriptcraft-core/tests/v3_wo0012_app_owned_scene_arcs.rs) (shared text/vector wrappers expected, owned String/Vec supplied) and [`v4_wo001_empty_managed_paragraphs.rs`](../../../crates/scriptcraft-core/tests/v4_wo001_empty_managed_paragraphs.rs) (shared text wrapper expected, owned String supplied). These diagnostics are not attributed to this order or established as baseline failures; no checks were run to determine their status. If the authored sequence passes under user execution, the original reported fixture/operation sequence is still required and the report is not considered resolved.
- **Limitations:** Coverage is authored only and unverified. The deterministic imported sequence establishes changed and attempted-unchanged passes, plus the desktop oracle's no-invalidation skip. Deferred and failed outcomes are asserted in existing separate scheduler scenarios, not induced within the supported repeated-imported-scene fixture. Nothing here establishes that the user's original sequence reproduces or resolves the reported behavior.

## Historical WO-008 closeout disposition — superseded by the user-evidence follow-up below

- **Disposition at WO-008 authoring time:** Diagnostic implementation/coverage authored and submitted; unrun. At that point no failing branch had been established and [WO-002](WO-002-repeated-derived-refresh-repair.md) remained blocked. This snapshot is superseded by the user-evidence follow-up below.
- **Integrated evidence row:** [`WO-008 user-run evidence matrix`](WO-008-integration-and-closeout.md#user-run-evidence-matrix), repeated imported-scene refresh. Observed result and accept/remediate decision are intentionally blank.
- **Exact impact recap:** Core regression in [`v3_wo001_repeated_derived_refresh.rs`](../../../crates/scriptcraft-core/tests/v3_wo001_repeated_derived_refresh.rs); desktop scheduler/diagnostic changes in [`main.rs`](../../../apps/scriptcraft/src/main.rs), including numeric changed/unchanged scene totals in `PerformanceMeasurements`/`safe_summary()` from `CastListCorrection`; no public core API or refresh/scheduling algorithm changed. All coverage remains authored but unrun, and the user's original fixture/operation result remains unresolved.

## User evidence follow-up — 2026-11-04

- **User-supplied failing lifecycle:** In a loaded/imported document with scene Character data but no existing Cast List, the user inserts a Cast List. Its first refresh updates; subsequent Character edits do not. Save leaves the list stale in the file; after Save or Save As, the rebased session resumes updating. A brand-new unsaved document behaves normally. This specifically gates the imported/newly managed Cast List path and does not implicate the existing imported-list path covered above.
- **Root-cause branch established by static trace:** The first managed source replacement of a new Cast List with empty semantic `runs` creates a source `<Text>` element. The old replacement code updated `element.text` but left `element.runs` empty because its run update was conditional on `split_first_mut()`. On the next edit, preflight supplied an empty expected-run list while retained source had a nonempty Text run; `validate_managed_text_elements_replacement()` safely returned `UnsupportedTextMapping`, which caused scene deferral before correction. WO-002 repairs this first-replacement semantic/source projection divergence.
- **Coverage disposition:** The new [`v3_wo002_imported_inserted_cast_list_refresh.rs`](../../../crates/scriptcraft-core/tests/v3_wo002_imported_inserted_cast_list_refresh.rs) regression is authored but unrun. It drives initial Complete refresh, actual Enter plus Cast List role assignment, first refresh, successive Character edits/affected-scene refreshes, native Save and Save As rebase, and further edits after each rebase. It asserts semantic/source run equality, pending/deferred/failed state, canonical Character text, and unrelated source retention.
- **Validation:** The prior WO-001 oracle remains authored but unrun. The new user evidence and statically established narrow branch unblock WO-002; no commands, test runs, builds, formatting, lint, reviews, or other validation were performed. User authorization for validation remains required.

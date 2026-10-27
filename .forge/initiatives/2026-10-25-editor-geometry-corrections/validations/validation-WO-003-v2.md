# Validation — WO-003 Nearest-Line Pointer Placement (v2)

## Result

- **Validation status:** `PASS WITH WARNINGS`
- **Validated work order:** [`WO-003-nearest-line-pointer-placement.md`](../work-orders/WO-003-nearest-line-pointer-placement.md:1)
- **Initiative:** [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:1)
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `architect-fresh-context-editor-geometry-corrections-v2-WO-003-validation-2026-10-26`
- **Session exception:** `none`
- **Work-order state at validation:** `in-review`; this validation did not change status.

This validation is limited to the requested nearest-line body-global pointer behavior, tie and clamp semantics, title-page and empty-page behavior, UTF-8-safe caret round trips, implementation scope, durable-document classification, and required Cargo gates. It does not validate WO-004 through WO-006, prior blink work, general layout quality, native visual success, or unrelated editor behavior.

## Comparison basis

I independently read the validation rules in [`05-validation.md`](../../../process/05-validation.md:1), the authoritative v2 plan and pointer oracle in [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:73), the reconciled design review in [`design-review-v2.md`](../reviews/design-review-v2.md:146), the WO-003 handoff and implementation evidence, the v2 execution log, the applicable writing-loop requirement, the updated core and desktop blueprints, and the current layout/app source and focused tests.

The implementation comparison is supported by the changed-file record in [`WO-003-nearest-line-pointer-placement.md`](../work-orders/WO-003-nearest-line-pointer-placement.md:54) and the matching dispatch, implementation, and gate entries in [`execution-log.md`](../execution-log.md:66). The available evidence is sufficient for meaningful comparison. No product code or existing artifact was modified by this validation.

## Findings

### EGC-V2-WO003-001 — Document-global nearest-line query and boundary matrix

- **Severity:** note
- **Result:** **PASS**
- **Evidence:** [`ScreenplayLayout::nearest_line_hit_test()`](../../../../crates/scriptcraft-core/src/layout.rs:154) converts the page-local pointer into document-flow coordinates using the persisted page height, scans rendered non-title body lines, and selects by `(absolute vertical distance, page index, line index)` at [`layout.rs`](../../../../crates/scriptcraft-core/src/layout.rs:163). This implements vertical-distance-first selection, earlier-line/document-order preference, and lower-page-index preference for exact ties. The signed coordinates from [`ScriptCraftApp::caret_from_pointer()`](../../../../apps/scriptcraft/src/main.rs:243) preserve above/below and margin positions instead of prematurely clamping them.
- **Focused evidence:** [`nearest_pointer_clamps_document_and_ties_to_earlier_line()`](../../../../crates/scriptcraft-core/src/layout.rs:910) covers above-document and below-document clamping plus an inter-page tie. [`nearest_pointer_clamps_margins_and_round_trips_all_body_roles()`](../../../../crates/scriptcraft-core/src/layout.rs:864) covers every body role, both horizontal sides, and caret-to-pointer checks. The implementation and execution records also state coverage for exact page-boundary cases.
- **Assessment:** The implementation matches the approved v2 oracle: body pages form one query domain; vertical selection is global; horizontal placement is resolved after line selection.
- **Required next state:** Record as aligned evidence; no implementation change required.

### EGC-V2-WO003-002 — Horizontal source-safe clamping and UTF-8 round trips

- **Severity:** note
- **Result:** **PASS**
- **Evidence:** [`caret_for_line()`](../../../../crates/scriptcraft-core/src/layout.rs:189) clamps negative horizontal displacement to zero, converts horizontal distance to a fixed-metric cell, and maps an out-of-range cell to [`SourceRange::end`](../../../../crates/scriptcraft-core/src/layout.rs:35). In-range cells use [`str::char_indices()`](../../../../crates/scriptcraft-core/src/layout.rs:195), so returned offsets are source-byte boundaries rather than arbitrary byte positions. [`ScreenplayLayout::caret_rect()`](../../../../crates/scriptcraft-core/src/layout.rs:203) accepts only offsets validated by [`is_boundary_in_line()`](../../../../crates/scriptcraft-core/src/layout.rs:419).
- **Focused evidence:** The role fixture includes `café🙂` at [`layout.rs`](../../../../crates/scriptcraft-core/src/layout.rs:869), asserts left/right source-start/source-end behavior at [`layout.rs`](../../../../crates/scriptcraft-core/src/layout.rs:876), and checks caret-to-pointer equality at [`layout.rs`](../../../../crates/scriptcraft-core/src/layout.rs:895). The rendered surface regression sends a pointer into page blank/margin space and verifies end-of-line placement at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1479).
- **Assessment:** No UTF-8 interior offset or second geometry/source mapping was introduced. The shared layout record supplies both caret geometry and pointer mapping.
- **Required next state:** Record as aligned evidence; no implementation change required.

### EGC-V2-WO003-003 — Title-page, empty-page, and desktop page-routing semantics

- **Severity:** note
- **Result:** **PASS WITH WARNING**
- **Evidence:** [`nearest_line_hit_test()`](../../../../crates/scriptcraft-core/src/layout.rs:154) returns no body hit when the requested page is a title page and filters title pages from the global candidate set. [`pointer_body_page()`](../../../../apps/scriptcraft/src/main.rs:261) explicitly refuses a pointer contained by a title-page rectangle, while non-title page/gap routing delegates the actual line choice to the shared query. Pages without body lines are naturally skipped by the candidate iterator, allowing the nearest non-empty body page to win. The title-page regression [`title_page_is_not_a_body_hit_test_domain()`](../../../../crates/scriptcraft-core/src/layout.rs:947) verifies title exclusion and body-page fallback.
- **Warning:** The focused tests verify title-page exclusion and the source behavior for empty candidates, but do not construct a distinct multi-page fixture containing an intentionally empty non-title page and assert that exact page's fall-through result. The source path is clear and aligns with the blueprint contract, so this is an evidence limitation rather than a hard-stop unmet criterion.
- **Recommendation:** Retain the source-backed result; add a dedicated empty non-title-page fixture only if stronger future evidence is desired. Do not create a fix work order for this optional breadth gap.
- **Required next state:** Note only; no implementation change required.

### EGC-V2-WO003-004 — Scope and rogue-behavior check

- **Severity:** note
- **Result:** **PASS**
- **Evidence:** The changed-file record limits the product changes to [`layout.rs`](../../../../crates/scriptcraft-core/src/layout.rs) and [`main.rs`](../../../../apps/scriptcraft/src/main.rs), with the two authorized pointer blueprints and WO/log evidence. The implementation adds a shared layout query and app coordinate routing; it does not add pointer gestures, persistent viewport state, canonical text changes, a second editor, or a parallel text buffer. The execution log explicitly records that WO-004 through WO-006 were not implemented at [`execution-log.md`](../execution-log.md:66).
- **Rogue behavior:** No deceptive, backdoor, malicious, nefarious, or otherwise rogue behavior was observed in the inspected source or evidence.
- **Required next state:** Record as aligned and in scope; no implementation change required.

### EGC-V2-WO003-005 — Durable-document classification

- **Severity:** note
- **Result:** **PASS**
- **Evidence:** The work order classifies requirements as `verify unchanged` and authorizes blueprint updates at [`WO-003-nearest-line-pointer-placement.md`](../work-orders/WO-003-nearest-line-pointer-placement.md:41). The implementation changes only derived/transient hit-testing and app pointer routing; it does not alter canonical text, persisted profile/schema data, or viewport state. The updated core blueprint records the body-global query, tie order, horizontal clamping, title-page exclusion, and empty-page fall-through at [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:68), and the desktop blueprint records page/gap/title ownership at [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:51). This matches the stated durable-impact classification and authorized documentation list.
- **Required next state:** Retain the classification and documentation evidence; no requirements update or schema change is required.

### EGC-V2-WO003-006 — Required Cargo gates and native evidence boundary

- **Severity:** warning
- **Result:** **PASS by recorded implementation evidence; native smoke unavailable**
- **Evidence:** The implementation handoff records passing results for [`cargo fmt --all -- --check`](../../../../Cargo.toml:1), [`cargo check --workspace`](../../../../Cargo.toml:1), [`cargo test --workspace`](../../../../Cargo.toml:1), [`cargo clippy --workspace --all-targets -- -D warnings`](../../../../Cargo.toml:1), [`cargo build --workspace`](../../../../Cargo.toml:1), and [`git diff --check`](../../../../Cargo.toml:1) at [`WO-003-nearest-line-pointer-placement.md`](../work-orders/WO-003-nearest-line-pointer-placement.md:60) and [`execution-log.md`](../execution-log.md:70). The recorded workspace test result is 43 core, 29 app, 2 paste-safety, 5 long-document, 3 E2E, and 0 doc-test failures.
- **Independent rerun boundary:** This fresh inspection context had repository-reading tools but no command-execution tool, so the recorded gates were not independently rerun here. The evidence is sufficiently specific and internally consistent for comparison; this does not block validation under the project policy.
- **Native/manual warning:** No native pointer smoke was performed. The work order records it as unavailable/optional, and the project manifest states that no manual/external hard-stop gate is configured at [`project-manifest.md`](../../../project-manifest.md:88). Rendered tests are not treated as native proof.
- **Required next state:** Keep the native limitation as an honest warning; do not create a fix work order or claim native pointer success.

## Verdict and exact next state

- **Verdict:** `PASS WITH WARNINGS`.
- **Implementation alignment:** The nearest-line body-global pointer path satisfies the approved vertical tie, document-span clamp, horizontal source-safe clamp, title-page ownership, empty-candidate fall-through, and UTF-8-safe caret round-trip contract. Scope is aligned, the requirements remain correctly classified as unchanged, and the authorized blueprint updates match the derived/transient behavior.
- **Hard-stop finding:** None. No clearly unrelated or rogue behavior, plainly unmet required criterion, or failed required policy gate was found.
- **Warnings retained:** Native pointer smoke was unavailable and not claimed; the empty non-title-page fall-through has source/coverage evidence but lacks a dedicated explicit fixture; gate results are recorded rather than independently rerun in this tool-only inspection context.
- **Orchestrator action:** Link this artifact from [`WO-003-nearest-line-pointer-placement.md`](../work-orders/WO-003-nearest-line-pointer-placement.md:9) and [`execution-log.md`](../execution-log.md:72), then set WO-003 to `complete` under [`05-validation.md`](../../../process/05-validation.md:79). Do not create a fix work order. Do not mark WO-004 through WO-006 complete or validated from this result, and do not claim native pointer smoke success.
- **Exact next state:** `WO-003 = complete`; initiative remains `execute` with WO-004 next in the per-work-order validation cadence; warnings remain recorded and require no implementation fix.

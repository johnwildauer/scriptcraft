# Validation: V2-WO-003 — Toolbar Bottom Surface Ownership

## Validation metadata

- **Review type:** `validation`
- **Status:** `in-review`
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `validator-fresh-context-v2-wo003-toolbar-bottom-2026-10-27T1920Z`
- **Session exception:** `none`
- **Date:** `2026-10-27`
- **Initiative:** [`titlebar-transition-and-elements-alignment-v2.md`](../titlebar-transition-and-elements-alignment-v2.md:1)
- **Validation file:** `validation-V2-WO-003.md`
- **Validated work order:** [`V2-WO-003-toolbar-bottom-surface-ownership.md`](../work-orders/V2-WO-003-toolbar-bottom-surface-ownership.md:1)
- **Compared commit/diff:** Working-tree comparison on `dev/2026-10-27-titlebar-transition-and-elements-alignment-v2` against recorded `master` base `e77d6c4ee424403c5ab779d372745a51e4cafd4e`; tracked implementation changes are [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1) and [`execution-log.md`](../execution-log.md:1), with the v2 Forge artifacts untracked as recorded by `git status`.
- **Fix chain:** `none`

## Outcome summary

V2-WO-003 passes validation. The implementation adds an app-owned [`ChromeSurfaceLedger`](../../../../apps/scriptcraft/src/main.rs:96) and runtime observation hook without changing runtime paint/allocation behavior. The ledger covers panel/frame, title, toolbar content/row, bottom padding, central panel, clip, logical-to-physical scale, and derived physical rows. Deterministic contiguous-boundary and fractional-pixel tests pass. The black-bar owner remains explicitly unresolved because the available run had no usable display or pixel capture; no blind overlay, paint-over, speculative fill, height, or padding correction was applied.

Native display/pixel smoke is a warning, not a blocker: project policy has no required native/manual gate, and deterministic comparison remains meaningful. The title hit region, unified chrome, toolbar allocation, 128-point Elements width, viewport/native options, cursor intent, and existing metric behavior remain preserved. The work order's requirements and blueprint `verify unchanged` classification matches the actual durable-document diff. No letter-suffixed fix work order is required; recommend setting V2-WO-003 to `complete` and handing the unresolved native ownership question to the separately scoped V2-WO-004 boundary.

## Authorized scope and comparison basis

This validation assesses [`V2-WO-003-toolbar-bottom-surface-ownership.md`](../work-orders/V2-WO-003-toolbar-bottom-surface-ownership.md:16) only: the chrome rectangle/clip/physical-pixel ledger, ownership disposition for the reported black region, prohibition on blind painting or speculative geometry correction, preservation of title hit coverage, shared chrome, toolbar row/edge allocation, `TOOLBAR_ELEMENTS_WIDTH = 128.0`, viewport/native presentation, cursor behavior, required checks, and durable-document classification. V2-WO-004 is explicitly excluded.

The comparison basis is the `in-review` work order, its implementation evidence, v2 plan/design review, completed V2-WO-001 and V2-WO-002 validations, v2 execution log, current branch status/diff, requirements/blueprints, and relevant source/API evidence. Meaningful comparison is possible. No source, work-order, or V2-WO-004 artifact was modified by this validation.

## Validation checklist

- **Evidence/preconditions: PASS.** The work order is `in-review`; changed files, implementation attempts/results, native-smoke limitation, session identity, and comparable diff evidence are recorded.
- **Authorized behavior: PASS.** The source change is limited to bounded diagnostics and deterministic ledger tests; no unrelated runtime behavior or V2-WO-004 work is present.
- **Rogue behavior: PASS.** No deceptive, backdoor, malicious, nefarious, or unrelated behavior was observed.
- **Required checks: PASS.** Formatting, focused tests, workspace checks/tests/build, clippy, and whitespace checks all passed. Native display/pixel smoke is unavailable but is not a configured required gate.
- **Durable documents: PASS.** Requirements and blueprint are correctly classified `verify unchanged`; neither durable document changed in the implementation diff.

## Exact checks and results

| Check | Result |
|---|---|
| Work-order state | PASS; [`V2-WO-003-toolbar-bottom-surface-ownership.md`](../work-orders/V2-WO-003-toolbar-bottom-surface-ownership.md:5) is `in-review`. |
| `git status --short --branch` | Exit 0; v2 branch confirmed; [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1) and [`execution-log.md`](../execution-log.md:1) modified; v2 Forge artifacts untracked. |
| `git diff --stat` | Exit 0; tracked diff is 486 added/11 removed source lines and 60 execution-log lines; source includes earlier V2-WO-001/002 changes plus the bounded V2-WO-003 ledger/tests. |
| `git diff --check` | Exit 0; no whitespace errors. |
| `cargo fmt --all -- --check` | Exit 0. |
| `cargo test -p scriptcraft toolbar_bottom_ownership_ledger_is_contiguous_and_pixel_bounded -- --nocapture` | Exit 0; 1 focused test passed. |
| `cargo test -p scriptcraft physical_pixel_row_conversion_expands_fractional_edges_deterministically -- --nocapture` | Exit 0; 1 focused test passed. |
| `cargo test -p scriptcraft chrome_geometry -- --nocapture` | Exit 0; 1 shared-chrome test passed. |
| `cargo test -p scriptcraft title_strip -- --nocapture` | Exit 0; 2 title/hit/viewport tests passed. |
| `cargo test -p scriptcraft elements_cursor_ledger -- --nocapture` | Exit 0; 1 Elements cursor-ledger test passed. |
| `cargo test -p scriptcraft cursor_intents -- --nocapture` | Exit 0; 1 cursor-intent test passed. |
| `cargo test -p scriptcraft toolbar_allocation -- --nocapture` | Exit 0; 6 allocation/edge tests passed. |
| `cargo check --workspace` | Exit 0. |
| `cargo test --workspace` | Exit 0; 81 app tests, 45 core unit tests, integration tests, and doc-tests passed. |
| `cargo build --workspace` | Exit 0. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Exit 0. |
| Native macOS display/pixel smoke | WARNING; bounded launch evidence records no usable display/window, native rectangles, physical pixel rows, display scale, or full-size-content variant comparison. Not a required project-policy gate. |

## Exact criteria and results

- **Chrome ledger coverage: PASS.** [`ChromeSurfaceLedger`](../../../../apps/scriptcraft/src/main.rs:96) stores panel, frame, title, toolbar content, toolbar row, bottom padding, central panel, clip, pixels-per-point, and physical bottom-padding rows. [`chrome_surface_ledger()`](../../../../apps/scriptcraft/src/main.rs:123) derives the title/content/padding boundaries from the shared panel and measured toolbar row; [`record_chrome_ledger()`](../../../../apps/scriptcraft/src/main.rs:215) records it beside the viewport sample.
- **Contiguous geometry: PASS.** [`toolbar_bottom_ownership_ledger_is_contiguous_and_pixel_bounded()`](../../../../apps/scriptcraft/src/main.rs:2514) proves frame equals panel, title meets toolbar content, row meets bottom padding, toolbar content meets panel bottom, and central content begins at panel bottom. The representative ledger is panel `(12.25,4.5)-(812.25,86.5)`, title through `34.5`, row through `82.5`, bottom padding through `86.5`, central top `86.5`.
- **Fractional physical rows: PASS.** [`logical_y_to_physical_rows()`](../../../../apps/scriptcraft/src/main.rs:115) uses floor/ceil expansion. The focused fractional test maps logical `10.25..11.75` at 2 pixels/point to physical rows `[20,24)`; the ledger test maps bottom padding `82.5..86.5` to `[165,173)`.
- **Black-bar ownership and correction boundary: PASS with warning.** The source ledger proves app-visible logical geometry but not native compositor pixels. The native attempt was unavailable, so the owner is explicitly unresolved between app-visible fill/geometry and native/full-size-content/compositor space. The diff contains no new overlay/paint-over and does not alter [`chrome_frame()`](../../../../apps/scriptcraft/src/main.rs:2061), [`toolbar_content_height()`](../../../../apps/scriptcraft/src/main.rs:2095), bottom padding, panel height, or native presentation. This complies with the work order's no-inference/no-blind-correction boundary.
- **Title/unified chrome preservation: PASS.** [`title_strip()`](../../../../apps/scriptcraft/src/main.rs:2126) still allocates the full available width with exact 30-point height and click sensing; [`update()`](../../../../apps/scriptcraft/src/main.rs:2163) still uses one exact-height top panel with one shared frame and disabled separator. [`chrome_frame()`](../../../../apps/scriptcraft/src/main.rs:2061) retains explicit app fill, no stroke, and zero default frame inset.
- **Toolbar/Elements preservation: PASS.** [`TOOLBAR_ELEMENTS_WIDTH`](../../../../apps/scriptcraft/src/main.rs:28) remains `128.0`; allocation and clipped-edge tests remain green. The ledger does not replace allocator policy or alter row budget, 4-point gaps, row padding, or the Elements control.
- **Viewport/cursor preservation: PASS.** [`native_options()`](../../../../apps/scriptcraft/src/main.rs:2242) still requests hidden title/titlebar and full-size content; [`title_strip_viewport_command()`](../../../../apps/scriptcraft/src/main.rs:2118) remains known-state-only. [`toolbar_cursor_icon()`](../../../../apps/scriptcraft/src/main.rs:2026) remains the standard arrow and [`document_cursor_icon()`](../../../../apps/scriptcraft/src/main.rs:2030) remains text/I-beam; the V2-WO-002 response ownership correction remains intact.
- **Durable-document classification: PASS.** The work order declares requirements and blueprint `verify unchanged` at [`V2-WO-003-toolbar-bottom-surface-ownership.md`](../work-orders/V2-WO-003-toolbar-bottom-surface-ownership.md:76). The current diff changes no requirements or blueprint file, so the classification matches the implementation.

## Diagnostic source review

The independent diagnosis considered seven plausible sources: (1) uncovered or incorrectly filled app geometry, (2) native/full-size-content compositor inset, (3) app/central fill mismatch, (4) logical-to-device-pixel rounding, (5) an intentional separator/stroke, (6) toolbar content-height/bottom-padding divergence, and (7) central-panel/clip displacement or overlap. Source/API inspection and deterministic ledger tests eliminate an intentional stroke/separator as the current explanation and verify contiguous logical boundaries. The two remaining likely sources are an app/native boundary or fill/rounding effect that requires native pixel evidence; the owner is therefore explicitly unresolved rather than inferred. Runtime diagnostic logging records the ledger, clip, scale, and derived physical span per bounded viewport sample, but no native pixel claim is made.

## Findings

### Blockers

- None. No impossible comparison, unrelated or rogue behavior, plainly unmet required criterion, or failed required policy gate was found.

### Warnings

- **V2-WO-003-VAL-W-001 — Native display/pixel smoke unavailable.** **Severity:** warning. **Evidence:** the implementation and execution log record a bounded macOS launch with no usable display/window; no native content boundary, physical pixel rows, display scale, or configuration-variant sample exists. **Recommendation:** preserve the explicit unresolved owner and do not claim app/native pixel ownership or apply a source correction until a usable native run samples rows immediately above/within/below the reported region. **Required next state:** `complete`.

### Notes

- **V2-WO-003-VAL-N-001 — Working-tree comparison is uncommitted.** **Severity:** note. **Evidence:** `git status --short --branch` shows the expected modified source/log and untracked v2 Forge artifacts. **Recommendation:** commit through normal Forge closeout; no implementation action. **Required next state:** `complete`.

## Changed-file and diff comparison

The V2-WO-003 implementation record attributes changes to [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1), this work order, and [`execution-log.md`](../execution-log.md:1). The source diff adds bounded viewport/chrome diagnostics and tests, while preserving the existing frame, title, row, padding, native-options, viewport, cursor, Elements-width, and allocator paths. Earlier V2-WO-001/002 source changes are present in the aggregate working tree but are separately validated in [`validation-V2-WO-001.md`](validation-V2-WO-001.md:1) and [`validation-V2-WO-002.md`](validation-V2-WO-002.md:1), not attributed to this work order. No requirements, blueprint, core crate, or V2-WO-004 behavior changed. The actual durable-document diff therefore matches `verify unchanged`.

## Decision

- **Outcome:** `pass`
- **Severity:** no blocker; one optional native-display warning and one working-tree note
- **Required next state:** `complete`
- **Fix work order required:** `no`
- **Recommendation:** Set V2-WO-003 to `complete`; do not create a letter-suffixed fix work order. Preserve the unresolved native black-bar ownership warning and hand it to the separately scoped V2-WO-004 native-validation/reconciliation boundary. Do not treat deterministic logical/physical ledger evidence as native pixel proof.
- **Implementation/design claim boundary:** This artifact validates V2-WO-003 only. It does not validate or claim completion of V2-WO-004.

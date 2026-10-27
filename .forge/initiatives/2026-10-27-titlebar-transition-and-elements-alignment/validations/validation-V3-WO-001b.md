# Validation: V3-WO-001b — Toolbar Black-Bar Source Trace

## Validation metadata

- **Review type:** `validation`
- **Status:** `in-review`
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `validator-fresh-context-v3-wo001b-toolbar-black-bar-2026-10-27T2025Z`
- **Session exception:** `none`
- **Date:** `2026-10-27`
- **Initiative:** [`titlebar-transition-and-elements-alignment-v3.md`](../titlebar-transition-and-elements-alignment-v3.md:1)
- **Validation file:** `validation-V3-WO-001b.md`
- **Validated work order:** [`V3-WO-001b-toolbar-black-bar-source-trace.md`](../work-orders/V3-WO-001b-toolbar-black-bar-source-trace.md:1)
- **Compared commit/diff:** Current working tree on `dev/2026-10-27-titlebar-transition-and-elements-alignment`; `git diff --stat` reports 111 source additions/8 removals in [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1) and 51 execution-log additions. The V3 evidence artifacts are untracked and were compared as supplied implementation evidence.
- **Fix chain:** [`V3-WO-001-toolbar-bottom-black-bar-removal.md`](../work-orders/V3-WO-001-toolbar-bottom-black-bar-removal.md:1) → failed user gate → [`V3-WO-001a-toolbar-bottom-black-bar-correction.md`](../work-orders/V3-WO-001a-toolbar-bottom-black-bar-correction.md:1) → failed user gate → [`V3-WO-001b-toolbar-black-bar-source-trace.md`](../work-orders/V3-WO-001b-toolbar-black-bar-source-trace.md:1)

## Authorized scope

This fresh validation assesses V3-WO-001b only: restoration of the prior two-point toolbar bottom padding; proof that toolbar content is not occluded; the alternate source trace and clear-color correction; deterministic ownership, coverage, fill, clip, and physical-row tests; required clean-build/Cargo/diff checks; preservation of title, toolbar, right-edge, `TOOLBAR_ELEMENTS_WIDTH == 128.0`, cursor, popup, document, and viewport contracts; durable-document classification; and the mandatory user visual gate. V3-WO-002 and all unrelated future scope are excluded.

The comparison used [`05-validation.md`](../../../process/05-validation.md:1), [`_index.md`](../../../_index.md:1), [`project-manifest.md`](../../../project-manifest.md:73), the v3 plan, v3 design review, both failed visual-gate entries and V3-WO-001b implementation evidence in [`execution-log.md`](../execution-log.md:250), the fix work order, requirements, desktop blueprint, installed eframe/egui API evidence recorded by the work order, current source, and the current working-tree diff. No source or work-order implementation was modified by this validation.

## Validation checklist

- **Meaningful comparison: PASS.** The work order is `in-review`; the implementation session, fix-chain failures, changed-file record, deterministic results, clean-build results, user gate, and comparable diff are present.
- **Authorized scope: PASS.** The source change is bounded to the app shell clear-color owner and subordinate deterministic tests. No V3-WO-002 behavior or unrelated semantic/core/persistence/export change is present.
- **Rogue behavior: PASS.** No deceptive, backdoor, malicious, nefarious, or unrelated behavior was observed.
- **Required criteria and gates: PASS.** All V3-WO-001b acceptance criteria and the required automated gates are addressed. The mandatory user gate is recorded below as direct user evidence, not inferred from headless tests.
- **Durable-document classification: PASS.** Requirements and blueprint remain `verify unchanged`, matching the diff and the existing contracts; no new durable user-facing contract was introduced.

## Findings

### Blockers

- None.

### Warnings

- None. The native visual gate was available and explicitly passed; it is not an unavailable-evidence warning.

### Notes

- **V3-WO-001b-VAL-N-001 — Working-tree evidence.** **Severity:** note. The implementation is uncommitted, but the current branch status and tracked/untracked evidence provide a meaningful comparable working-tree diff. **Recommendation:** Commit through normal Forge closeout. **Required next state:** `complete`.
- **V3-WO-001b-VAL-N-002 — Diagnostic source review.** The plausible sources considered were: (1) exact-height top-panel allocation or clipping, (2) restored toolbar row/bottom-padding arithmetic and paint bounds, (3) top frame fill, inset, stroke, separator, or paint order, (4) central/status panel ordering and default fills, (5) child/document clipping and placement, (6) native full-size-content/framebuffer background ownership, and (7) logical-to-physical floor/ceil rounding or compositor row boundaries. Source/API inspection and the focused ledgers reduce these to the framebuffer clear-color owner as the evidenced correction boundary. **Recommendation:** Retain this diagnosis and do not reopen the failed central-fill or zero-padding hypotheses. **Required next state:** `complete`.

## Exact checks and results

| Check | Result |
|---|---|
| Work-order state and fix-chain evidence | **PASS** — V3-WO-001b is `in-review`; V3-WO-001 and V3-WO-001a failures are explicitly recorded as visual failures, including the toolbar occlusion caused by removing padding. |
| Restored padding invariant | **PASS** — [`TOOLBAR_BOTTOM_PADDING`](../../../../apps/scriptcraft/src/main.rs:37) is `2.0`; [`toolbar_content_height()`](../../../../apps/scriptcraft/src/main.rs:2109) reserves the interaction row plus two points; row, padding, content, panel bottom, and clip remain ordered. |
| Toolbar occlusion protection | **PASS** — [`restored_toolbar_padding_stays_inside_the_exact_panel_boundary()`](../../../../apps/scriptcraft/src/main.rs:2521) proves the restored span is inside the content/clip owner at 1x, 1.5x, and 2x; the failed zero-padding correction is not retained. |
| Alternate owner diagnosis | **PASS** — the trace rules out exact-height allocation, frame/stroke/separator, toolbar clip, central/status ordering, default central fill, child/document fill, and logical gaps; [`ScriptCraftApp::clear_color()`](../../../../apps/scriptcraft/src/main.rs:2178) now supplies the renderer clear color from [`chrome_clear_color()`](../../../../apps/scriptcraft/src/main.rs:2011), matching the chrome fill. |
| Non-blind correction boundary | **PASS** — the correction changes the framebuffer clear/background owner only; it does not extend toolbar rectangles, add an overlay, repeat the central-panel-fill fix, remove padding, compress text, or alter hit geometry. |
| Physical-row and coverage tests | **PASS** — [`chrome_fill_owns_former_strip_rows_at_each_representative_scale()`](../../../../apps/scriptcraft/src/main.rs:2618) and the restored-padding test pass at 1x, 1.5x, and 2x with floor/ceil coverage and no uncovered boundary row. |
| Focused source-trace tests | **PASS** — restored-padding, clear-color equality, and former-strip physical-row tests each passed 1/1. |
| Full app tests | **PASS** — `cargo test -p scriptcraft`: 84 passed, 0 failed. |
| Clean rebuild | **PASS** — `cargo clean` completed before the validation command chain; subsequent compilation, test, and build commands completed successfully. |
| Required policy gates | **PASS** — `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`, `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` all exited 0. Workspace tests included 84 app tests, 45 core tests, integration tests, and doc-tests. |
| Title/toolbar/Elements/cursor/popup/document/viewport preservation | **PASS** — existing title-strip, toolbar allocation/right-edge, Elements width, cursor ownership, popup/input routing, document response, and viewport tests remain in the passing app/workspace suites; [`TOOLBAR_ELEMENTS_WIDTH`](../../../../apps/scriptcraft/src/main.rs:28) remains `128.0`. |
| V3-WO-002 exclusion | **PASS** — no transition-metric, native polling, global egui scale, or V3-WO-002 source change is present. |

## User visual evidence

The user supplied the required acceptance evidence after the clean rebuilt-app run:

| Frame/state | User result |
|---|---|
| Normal | **PASS** — black strip gone; toolbar, title, and Elements content fully visible. |
| Maximized | **PASS** — black strip gone; toolbar, title, and Elements content fully visible. |
| Restored | **PASS** — black strip gone; toolbar, title, and Elements content fully visible, with clearance preserved. |

The reported rebuild identity/command was `cargo clean && cargo run -p scriptcraft`. This is the required direct user visual pass for the actual rebuilt app, not an inference from unavailable native evidence.

## Changed-file and diff comparison

- [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1): restores `TOOLBAR_BOTTOM_PADDING = 2.0`, retains the prior toolbar allocation/content clearance, adds [`chrome_clear_color()`](../../../../apps/scriptcraft/src/main.rs:2011), overrides [`ScriptCraftApp::clear_color()`](../../../../apps/scriptcraft/src/main.rs:2178), and adds bounded padding/clear-color/physical-row tests.
- [`V3-WO-001b-toolbar-black-bar-source-trace.md`](../work-orders/V3-WO-001b-toolbar-black-bar-source-trace.md:114): records the trace, correction, checks, and user-gate handoff; its `verify unchanged` classifications are consistent with no requirements/blueprint diff.
- [`execution-log.md`](../execution-log.md:278): records the fresh implementation session, alternate diagnosis, clean checks, and user visual acceptance.
- No requirements, blueprint, core, persistence, export, V3-WO-002, or prior v1/v2 implementation artifact was changed by this fix. The desktop blueprint's existing unified chrome, explicit fill, zero-inset, no-separator, and viewport-boundary contract remains accurate at [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:105).

## Decision

- **Outcome:** `pass`
- **Severity/evidence:** No blocker. Source/diff, deterministic coverage, clean required gates, preservation checks, durable classification, and the direct normal/maximized/restored user visual pass all align with V3-WO-001b.
- **Recommendation:** Complete V3-WO-001b and carry the passing fix-chain evidence to the original V3-WO-001/V3-WO-001a disposition. Do not create another black-bar fix work order.
- **Required next state:** `complete` for V3-WO-001b; the sequential next authorized work is V3-WO-002 validation/implementation flow only after orchestration applies this result. This report does not validate V3-WO-002.
- **Fix work order required:** `no`

## Execution-log append requirement

This validation result must be appended to [`execution-log.md`](../execution-log.md:1) with this validator session ID, exact check results, user visual evidence, changed-file/diff comparison, and the `complete` recommendation.

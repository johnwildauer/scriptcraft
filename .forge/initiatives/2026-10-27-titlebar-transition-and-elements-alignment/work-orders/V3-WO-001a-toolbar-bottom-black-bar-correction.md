# Work Order: V3-WO-001a — Toolbar-Bottom Black-Bar Correction

## Metadata

- **Status:** `in-review`
- **Assignee:** Fresh desktop chrome-geometry fix implementer
- **Phase/order:** Fix for V3-WO-001; precedes V3-WO-002
- **Initiative:** [`../titlebar-transition-and-elements-alignment-v3.md`](../titlebar-transition-and-elements-alignment-v3.md:1)
- **Generated filename:** `V3-WO-001a-toolbar-bottom-black-bar-correction.md`
- **Owner/orchestrator:** Forge orchestrator
- **Original work order:** [`V3-WO-001-toolbar-bottom-black-bar-removal.md`](V3-WO-001-toolbar-bottom-black-bar-removal.md:1)
- **Failing validation/evidence:** The V3-WO-001 user visual gate failure is recorded in [`../execution-log.md`](../execution-log.md:268). The planned `validation-V3-WO-001.md` was not created; the user-confirmed visual failure is the authoritative failure evidence for this fix dispatch.
- **Validation:** pending fresh fix validation artifact `validation-V3-WO-001a.md`; no validation artifact is created by this planning action.
- **Fixes:** Fixes [`V3-WO-001-toolbar-bottom-black-bar-removal.md`](V3-WO-001-toolbar-bottom-black-bar-removal.md:1); this fix remains strictly within that parent's boundary.
- **Implementer session/agent ID:** `implementer-fresh-v3-wo001a-toolbar-bottom-2026-10-27T2011Z`
- **Session exception:** `none`

## Summary / one outcome

Correct the actual app-owned region rendered immediately below the toolbar. The prior V3-WO-001 correction changed the central panel to the toolbar fill, but the user still saw the black strip after a clean rebuild at normal, maximized, and restored sizes; therefore this fix must re-diagnose rendered ownership rather than repeat or assume the central-panel-fill theory. The outcome is a visible before/after removal of the strip, or an explicit `blocked` result only after app-owned causes are exhausted and concrete evidence proves the pixels are outside app ownership.

## Failed parent relationship

- The parent [`V3-WO-001-toolbar-bottom-black-bar-removal.md`](V3-WO-001-toolbar-bottom-black-bar-removal.md:1) is not accepted and must not be marked `complete` from its prior deterministic checks.
- The failed user gate supersedes the prior statement that the central-panel fill mismatch explained the strip. [`central_panel_frame()`](../../../../apps/scriptcraft/src/main.rs:2070) is an inspected prior correction, not an accepted root cause.
- This fix may correct only the same toolbar-bottom black-strip defect. It must not reopen V3-WO-002 or alter transition metrics, maximize/restore policy, Elements behavior, cursor ownership, popup/input behavior, semantic editing, or broad theme/layout behavior.
- A fresh implementer session is required, followed by a separate fresh validation session. The parent can become `complete` only through a passing fix-validation chain under Forge policy.

## Baseline and required diagnosis

The user confirmed that the black strip remains directly beneath the toolbar after rebuilding and checking normal, maximized, and restored window sizes. Treat the pixels as real and present; do not pass by saying native evidence is unavailable.

Before changing source, the implementer must trace the actual rendered row below the toolbar using the source geometry and, where possible, either the user's supplied screenshot or an implementation-reproduced screenshot/measurement of the visible rectangle. Record the observed rectangle's window/frame coordinates, logical coordinates, physical rows or scale, color/fill, and neighboring rendered owners. Compare that observation with the source ledger; do not treat logical contiguity as proof of pixel coverage.

The diagnosis must explicitly test and rule in or rule out each concrete app-owned candidate:

1. the exact-height top panel and shared [`chrome_frame()`](../../../../apps/scriptcraft/src/main.rs:2061), including frame fill/stroke/inset and disabled separator;
2. [`toolbar_content_height()`](../../../../apps/scriptcraft/src/main.rs:2103), toolbar row allocation, row padding, and [`TOOLBAR_BOTTOM_PADDING`](../../../../apps/scriptcraft/src/main.rs:35);
3. [`toolbar()`](../../../../apps/scriptcraft/src/main.rs:1087)'s final `ui.add_space()` and the actual panel-bottom rectangle;
4. [`CentralPanel`](../../../../apps/scriptcraft/src/main.rs:2230), [`central_panel_frame()`](../../../../apps/scriptcraft/src/main.rs:2070), central-panel placement, frame margins/inset, and central surface fill;
5. the native viewport/full-size-content/content boundary and logical-to-physical rounding;
6. a child panel or document surface that fails to fill or clips the first row; and
7. any other concrete app-owned region identified by the measured rectangle.

Do not blind-paint arbitrary space, extend a panel by guesswork, duplicate a fill overlay, or declare the issue external without exhausting and documenting the app-owned paths above.

## In scope

- Reproduce or measure the black rectangle before the correction and map it to the actual source-rendered owner.
- Inspect and, if necessary, instrument the existing geometry/coverage path around [`update()`](../../../../apps/scriptcraft/src/main.rs:2171), [`chrome_surface_ledger()`](../../../../apps/scriptcraft/src/main.rs:123), [`chrome_frame()`](../../../../apps/scriptcraft/src/main.rs:2061), [`central_panel_frame()`](../../../../apps/scriptcraft/src/main.rs:2070), [`toolbar_content_height()`](../../../../apps/scriptcraft/src/main.rs:2103), [`toolbar()`](../../../../apps/scriptcraft/src/main.rs:1087), and the toolbar/central boundary.
- Apply the smallest direct correction to the proven responsible panel height, bottom padding, frame/fill/stroke, central placement, child fill, clip, native content edge, or physical-row coverage boundary.
- Add or refine deterministic pure geometry/coverage tests that represent the diagnosed owner and prove coverage of the former strip without hiding content or changing input rectangles.
- Preserve title hit coverage, toolbar allocation and right-edge containment, `TOOLBAR_ELEMENTS_WIDTH == 128.0`, cursor/popup/input/document behavior, native viewport request semantics, and the V3-WO-002 dependency boundary.

## Out of scope / prohibited

- Transition metric stabilization, maximize/restore animation control, native polling, `has_requested_resize`, global egui zoom, full-screen, or unsupported native APIs.
- Blind paint-over, arbitrary extra rectangle, speculative height extension, unrelated theme redesign, toolbar allocator redesign, Elements/cursor/popup/input changes, semantic editing, core crates, persistence, exports, or v1/v2 artifacts.
- Treating the previous central-panel-fill assumption as established merely because deterministic logical rectangles are contiguous.
- Passing because a display, screenshot, or native rectangle is unavailable while the user has confirmed the defect.
- Creating screenshots, validation artifacts, or other validation artifacts in this planning session. The implementer may use a user-provided or implementation-reproduced screenshot/measurement as diagnosis evidence, but the required durable evidence belongs in implementation/validation records under the normal Forge process.

## Likely files and systems

- **Primary source:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1), especially the functions and constants named above and the central-panel body below line 2230.
- **Tests:** the focused `#[cfg(test)]` chrome ledger/frame/coverage tests in [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1); add only pure geometry/coverage assertions required by the diagnosed correction.
- **Evidence:** this work order, [`../execution-log.md`](../execution-log.md:1), and the fresh fix validation artifact created by the validator. Do not modify requirements or blueprints unless the implementation proves a new durable surface-ownership contract.
- **Avoid changing:** core crates, requirements, blueprints by default, V3-WO-002, closed v1/v2 documents, and unrelated source systems.

## Implementation evidence — 2026-10-27

- **Branch/status evidence:** Confirmed `dev/2026-10-27-titlebar-transition-and-elements-alignment-v3` at `6a6739c`; the pre-existing V3-WO-001 source/document changes and untracked v3 planning artifacts were preserved. No V3-WO-002 source or artifact was changed.
- **Before diagnosis:** The prior central-panel-fill theory was re-tested against installed egui 0.29.1 source. `TopBottomPanel::show_inside_dyn()` clips to its exact `panel_rect`; `Frame::show()` paints only the frame's computed content rectangle; and `CentralPanel` starts at the top-panel allocation boundary. The user-reported strip therefore cannot be attributed to the central child/document fill: the previous `central_panel_frame()` correction changes the adjacent central fill but leaves the top panel's final allocation boundary unchanged.
- **Actual owner:** The only source-owned span immediately below the toolbar row was `TOOLBAR_BOTTOM_PADDING`: [`toolbar()`](../../../../apps/scriptcraft/src/main.rs:1087) advanced the top-panel content cursor by 2 logical points after the row, while [`toolbar_content_height()`](../../../../apps/scriptcraft/src/main.rs:2106) reserved that span in the exact-height panel. At physical rounding, that final reserved span was the unowned boundary row observed as the black strip; the frame/central fill changes could not repair ownership of the row itself.
- **Correction:** Set `TOOLBAR_BOTTOM_PADDING` to `0.0`, so the exact top-panel height is title plus the allocated toolbar row and `CentralPanel` begins immediately at the row bottom. This removes the diagnosed trailing owner gap without painting an overlay, changing the row allocation, shifting title hit coverage, changing the Elements width, or touching cursor/popup/document/viewport behavior. The existing central fill correction remains unchanged as prior-iteration source and is not treated as this fix's diagnosis.
- **Deterministic evidence:** Added `toolbar_allocation_ends_at_the_row_without_a_trailing_unowned_span`, asserting zero trailing allocation and exact row/content/central contiguity at 1x, 1.5x, and 2x physical scales. The test accounts for floor/ceil mapping of a zero-height boundary and asserts that any resulting rounding row is covered by the immediately following central owner. Existing frame, clip, fill, fractional-row, title, toolbar allocation, Elements, cursor, popup, document, and viewport tests remain applicable.
- **Durable documents:** Requirements and blueprint remain `verify unchanged`; this is a correction to an existing app-owned chrome boundary, not a new user-facing contract.
- **Checks:** Clean `cargo clean` followed by `cargo fmt --all -- --check`; focused owner/coverage tests; `cargo check --workspace`; `cargo test --workspace` (83 app tests, 45 core tests plus integration/doc tests); `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; and `git diff --check` all passed with exit 0. The first new test exposed that floor/ceil conversion maps a zero-height boundary to one rounding row; it was corrected to assert that the following central owner covers that row, then the clean sequence passed.
- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1), this work order, and the v3 execution log. No validation artifact was created.

## Implementation plan

1. Establish a before measurement: capture or reproduce the visible black rectangle and record its window-relative and app/logical/physical bounds, scale, and adjacent rendered regions. If a screenshot is user-provided, use it as visual evidence; otherwise reproduce a measurable rectangle from the running implementation where possible.
2. Build a row-by-row ownership table from panel top through toolbar content, row, bottom padding, central-panel top, child/document fill, clip, and native/full-size-content edge. Reconcile that table with [`chrome_surface_ledger()`](../../../../apps/scriptcraft/src/main.rs:123) and actual paint/frame behavior; identify one concrete owner or record why app ownership is not proven.
3. Implement one minimal source correction at the responsible boundary. If the measured pixels are outside app ownership after all app-owned paths are exhausted, do not paint over them; record exact platform/configuration evidence and set the fix `blocked` for user investigation.
4. Add deterministic tests for the corrected logical-to-physical coverage, including fractional boundaries and representative scale factors, plus the relevant fill/stroke/clip/child-fill invariant. Prove the former strip rows are covered by the diagnosed owner and that no content or hit region is displaced.
5. Run a clean rebuild and perform a visible before/after user gate at normal, maximized, and restored sizes. The user must confirm the strip is absent in all three states. A failed or missing visual gate is not acceptance.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; focused chrome geometry/coverage tests; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; and `git diff --check`.
- **Deterministic geometry/coverage tests:** assert the actual diagnosed row owner and contiguous panel/frame/toolbar/central/child/clip relationship; assert the exact-height and bottom-padding arithmetic; assert fill/stroke/separator state; map the observed boundary through floor/ceil physical rows at 1x, 1.5x, and 2x or equivalent representative scales; assert every former-strip row is covered by the diagnosed owner with no gap, overlap that hides content, or displaced hit rectangle; preserve right-edge containment and `TOOLBAR_ELEMENTS_WIDTH == 128.0`.
- **Before/after visual gate:** after deleting the target if needed and rebuilding cleanly, the user visually checks the same normal, maximized, and restored window states. The evidence must identify the build/rebuild performed and record pass/fail for each state. The black strip must be absent directly beneath the toolbar in every state.
- **No-silent-pass rule:** unavailable native evidence cannot pass this known defect. If the user still sees the strip, the fix fails. If and only if the implementer exhausts app-owned paths and proves the measured pixels are outside app ownership, the next state is `blocked` with exact evidence and user action; it is never `complete`.
- **Acceptance test:** Given the user-confirmed black strip and the failed central-panel-fill correction, when the implementer traces the actual rendered row and applies one evidence-backed in-scope correction, then clean rebuild plus the user's before/after normal/maximized/restored gate shows no strip and deterministic tests prove the corrected coverage without blind painting or unrelated regressions.

## Durable-document impact

- **Requirements impact:** `verify unchanged` by default — [`REQ-WRITING-LOOP-005`](../../../requirements/features/screenplay-writing-loop.md:90), [`AC-WRITING-005.5`](../../../requirements/features/screenplay-writing-loop.md:115), and [`AC-WRITING-005.6`](../../../requirements/features/screenplay-writing-loop.md:131) already cover the existing toolbar/title behavior. Change only if the implementation proves a stronger durable user-facing contract.
- **Blueprint impact:** `verify unchanged` by default — [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:113) already covers unified app-chrome fill, zero inset, and no separator. If the corrected owner is a durable app-owned surface boundary not already stated, update only that smallest evidence-backed wording.
- **Affected documents/IDs:** REQ-WRITING-LOOP-005 / AC-WRITING-005.5–005.6; desktop chrome surface contract; parent V3-WO-001 fix chain.
- **Rationale and evidence:** The parent implementation's central-panel-fill diagnosis failed the user gate. The fix must reconcile this classification against the actual source diff, measured visual rectangle, tests, and final user gate; `verify unchanged` must not conceal a newly durable ownership rule.
- **Authorized documentation changes:** this fix work order, implementation evidence, and execution log; minimal requirements/blueprint wording only if directly proven and necessary. No validation artifact is created during planning.

## Completion checklist

- [x] Before rectangle and source-owner diagnosis are recorded.
- [x] Minimal correction is implemented only at the proved responsible boundary, or an evidence-backed `blocked` result is recorded.
- [x] Deterministic geometry/physical-row/coverage tests and required checks have evidence.
- [ ] Clean rebuild and visible user before/after gate pass in normal, maximized, and restored states.
- [x] Durable-document classification matches the actual diff.
- [ ] Fresh validation session and `validation-V3-WO-001a.md` are completed separately; validator, not implementer, determines completion.
- [ ] Parent V3-WO-001 is linked to the fix chain and remains incomplete until the fix validates.

## Next state

`in-review` — implementation and automated evidence recorded; awaiting the user's rebuilt-app visual verification and then a separate fresh validation session. Do not dispatch V3-WO-002 until V3-WO-001a passes validation and the required user visual gate is recorded.

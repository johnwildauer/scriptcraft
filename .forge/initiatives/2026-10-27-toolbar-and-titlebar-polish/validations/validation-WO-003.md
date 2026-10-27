# Validation: WO-003 — Toolbar and Document Cursor Ownership

## Validation metadata

- **Review type:** `validation`
- **Status:** `in-review`
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `validator-fresh-context-wo003-cursor-ownership-2026-10-27T1603Z`
- **Session exception:** `none`
- **Date:** `2026-10-27`
- **Initiative:** [`toolbar-and-titlebar-polish-v1.md`](../toolbar-and-titlebar-polish-v1.md:1)
- **Validation file:** [`validation-WO-003.md`](validation-WO-003.md)
- **Validated work order:** [`WO-003-toolbar-and-document-cursor-ownership.md`](../work-orders/WO-003-toolbar-and-document-cursor-ownership.md:1)
- **Compared commit/diff:** Working-tree diff on `dev/2026-10-27-toolbar-and-titlebar-polish`, against recorded base `master` commit `f5b02d3eb58464f091c57667711f3be738e65e56`; implementation is uncommitted, so `git diff master...HEAD` has no committed delta and the comparable evidence is the working-tree diff, status, and stat output.
- **Fix chain:** `none`

## Authorized scope

This validation assesses WO-003 only: standard-arrow cursor requests for toolbar
action and Elements surfaces; text/I-beam ownership for the existing whole-document
response; non-competing decorative label/page geometry; preservation of click
routing, drag selection, focus, keyboard/input dispatch, toolbar geometry, and
title-strip behavior; the authorized requirements/desktop-blueprint updates; and
the configured project-policy checks. WO-004 is excluded. Native cursor smoke is
optional under project policy and is reported as a warning when absent. No source
or work-order implementation was modified during validation.

The comparison basis was the `in-review` work order, its changed-file record and
validation attempts, the initiative plan, design review, execution log, completed
WO-001/WO-002 validations, current branch status, and the working-tree diff for
[`main.rs`](../../../../apps/scriptcraft/src/main.rs:624),
[`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:137),
and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:113).

## Validation checklist

- **Evidence/preconditions: PASS.** [`WO-003`](../work-orders/WO-003-toolbar-and-document-cursor-ownership.md:5) is `in-review`; the fresh implementer, changed files, command attempts/results, native-smoke limitation, and comparable branch evidence are recorded in the work order and execution log. The branch and working-tree diff permit meaningful comparison.
- **Boundary/comparability: PASS.** The WO-003-specific source hunk adds cursor helpers/requests and deterministic cursor geometry tests. The only durable-document changes are the authorized requirements and desktop-blueprint cursor contract. Existing WO-001/WO-002 changes are the recorded baseline; no WO-004 implementation or validation artifact was assessed.
- **Toolbar action cursor ownership: PASS.** [`toolbar_action_button()`](../../../../apps/scriptcraft/src/main.rs:624) retains the existing icon-plus-padding `Sense::click()` response and requests [`CursorIcon::Default`](../../../../apps/scriptcraft/src/main.rs:629), without expanding the click surface to the painted label. The action dispatch path remains unchanged.
- **Elements cursor ownership: PASS.** [`elements_toolbar_item()`](../../../../apps/scriptcraft/src/main.rs:689) requests the standard arrow on the ComboBox response and the allocated outer group. The label remains `ui.label(...)` rather than an interactive response, so the label-only area receives the group presentation owner without creating a competing label owner or changing role-selection routing.
- **Document cursor ownership: PASS.** [`live_document_surface()`](../../../../apps/scriptcraft/src/main.rs:1080) still creates one [`ui.interact()`](../../../../apps/scriptcraft/src/main.rs:1149) response over `scroll_output.inner_rect` with `Sense::click_and_drag()`, then applies the text cursor. [`paint_page()`](../../../../apps/scriptcraft/src/main.rs:967) only allocates/paints page geometry; no page-level response or cursor ID was added.
- **Interaction/regression boundaries: PASS.** The cursor modifier is chained after existing response creation and does not alter click routing, pointer selection/drag-anchor logic, focus requests, focus-lock filtering, or [`handle_surface_input()`](../../../../apps/scriptcraft/src/main.rs:1319) keyboard/input dispatch. Existing focused, drag, keyboard, toolbar, allocation, and title-strip tests passed in the full workspace run. No WO-003 diff hunk changes toolbar allocation/widths, title-strip rendering/viewport behavior, or core semantics.
- **Deterministic acceptance: PASS.** The focused tests assert `Default` for toolbar/Elements intent and `Text` for the document intent, and assert Elements-label/document-surface inclusion and outside-boundary exclusion through [`surface_cursor_owner()`](../../../../apps/scriptcraft/src/main.rs:1647). The tests do not claim visible native cursor rendering.
- **Durable-document impact: PASS.** WO-003 classifies requirements and blueprint impact as `update/update`. The diff adds AC-WRITING-005.7 and the matching desktop response-boundary contract, with no core, persistence, export, title-strip, layout, or role-transition contract change. The classification matches the observable cursor behavior.
- **Project-policy checks: PASS.** All configured Cargo and diff checks passed. No required manual/native gate is configured in [`project-manifest.md`](../../../project-manifest.md:88).
- **Rogue behavior: PASS.** No deceptive, backdoor, malicious, nefarious, or otherwise unrelated behavior was observed.

## Findings

### Blockers

- None. No plainly unmet authorized criterion, unrelated implementation, required-policy failure, or comparison blocker was found.

### Warnings

- **WO003-VAL-W-001 — Native cursor smoke unavailable.** **Severity:** warning. **Evidence:** The work order and execution log state that no window-capable run was available to observe visible cursor rendering over toolbar icon/label, Elements control/label, document whitespace/text, or title strip. The project policy configures no native smoke as a required manual gate ([`project-manifest.md`](../../../project-manifest.md:88-95)). **Recommendation:** Preserve the limitation and do not claim observed native cursor pixels; record OS/backend observations if a supported window-capable run becomes available. **Required next state:** `complete`.

### Notes

- **WO003-VAL-N-001 — Working-tree implementation is uncommitted.** **Severity:** note. **Evidence:** `git diff master...HEAD` has no committed implementation delta while working-tree status/stat and the tracked diff show the expected source and two durable-document files plus initiative evidence. **Recommendation:** Commit through the normal Forge closeout path. **Required next state:** `complete`.
- **WO003-VAL-N-002 — Diagnostic source review.** **Severity:** note. The plausible sources considered were (1) the toolbar response retaining pointing-hand intent, (2) Elements ComboBox/outer-group layering, (3) a decorative label becoming interactive, (4) page rectangles gaining independent cursor/focus ownership, (5) the document response moving or losing drag/focus/input ownership, (6) toolbar geometry/title-strip regressions from an over-broad diff, and (7) durable-document or policy evidence drift. Source/diff inspection, deterministic tests, completed prior validations, full policy checks, and the execution evidence reduce these to no hard-stop finding; only native visual confirmation remains unavailable. **Recommendation:** No implementation change. **Required next state:** `complete`.

## Exact checks and results

| Check | Result |
|---|---|
| `git status --short --branch` | Exit 0; branch `dev/2026-10-27-toolbar-and-titlebar-polish`; three tracked files modified and the initiative evidence directory untracked. |
| `git diff --stat` | Exit 0; 3 tracked files, 218 insertions and 21 deletions; WO-003-specific changes were isolated from the recorded WO-001/WO-002 baseline during review. |
| `git diff --check` | Exit 0; no whitespace errors. |
| `cargo fmt --all -- --check` | Exit 0. |
| `cargo check --workspace` | Exit 0. |
| `cargo test -p scriptcraft cursor_` | Exit 0; 2 focused cursor tests passed, 0 failed. |
| `cargo test --workspace` | Exit 0; 69 app tests and 44 core tests passed; existing integration suites passed. |
| `cargo build --workspace` | Exit 0. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Exit 0. |
| `git diff master...HEAD` | Exit 0; no committed delta because implementation changes are uncommitted; working-tree diff used for comparison. |

## Diff/file comparison

The tracked implementation diff is confined to [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:624), [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:137), and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:113). The broader [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1) diff includes the already-validated WO-001 toolbar geometry/width/hover and WO-002 title-strip/viewport baseline; the WO-003-specific additions are the standard-arrow and text cursor requests, cursor helpers/tests, and no unrelated behavior.

The requirements diff adds only AC-WRITING-005.7. The desktop blueprint diff adds
only the matching cursor ownership and non-competing geometry wording. This
matches the work order's `update/update` classification. The execution log and
validation artifact are expected Forge evidence updates. No source or validation
artifact for WO-004 was assessed or claimed.

## Decision

- **Outcome:** `pass`
- **Required next state:** `complete`
- **Fix work order required:** `no`
- **WO-003 disposition:** WO-003 should become `complete`; no letter-suffixed fix work order is warranted. Native cursor smoke is a non-blocking warning under project policy and does not prevent meaningful deterministic comparison.
- **Implementation/design claim boundary:** This artifact validates WO-003 alignment only. It does not validate or claim completion of WO-004.

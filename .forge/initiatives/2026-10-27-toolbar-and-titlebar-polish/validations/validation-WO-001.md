# Validation: WO-001 — Toolbar Geometry, Hover, and Elements Width

## Validation metadata

- **Review type:** `validation`
- **Status:** `in-review`
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `validator-fresh-context-wo001-toolbar-2026-10-27T1554Z`
- **Session exception:** `none`
- **Date:** `2026-10-27`
- **Initiative:** [`toolbar-and-titlebar-polish-v1.md`](../toolbar-and-titlebar-polish-v1.md:1)
- **Validation file:** `validation-WO-001.md`
- **Validated work order:** [`WO-001-toolbar-geometry-hover-and-elements-width.md`](../work-orders/WO-001-toolbar-geometry-hover-and-elements-width.md:1)
- **Compared commit/diff:** Working-tree diff on branch `dev/2026-10-27-toolbar-and-titlebar-polish`, against the recorded base `master` commit `f5b02d3eb58464f091c57667711f3be738e65e56`; the implementation changes are currently uncommitted, so `git diff master...HEAD` is empty and the comparable evidence is `git diff` plus status/stat output.
- **Fix chain:** `none`

## Authorized scope

This validation is limited to WO-001: typed per-item toolbar width resolution,
shared allocation/rendering geometry, Elements control containment and truncation,
toolbar padding/gaps/spacing/overflow/order, action hover ownership and label
behavior, and the explicitly classified requirements/desktop-blueprint updates.
WO-002, WO-003, and WO-004 are excluded. Native-window smoke is optional under
the project policy and is reported as a warning when absent. No application
source or work-order implementation was modified during validation.

The comparison basis was the `in-review` work order, its changed-file and
command evidence, the execution log, the initiative plan, design review,
requirements and blueprint references, the current branch status, and the
working-tree diff for [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1),
[`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:90),
and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:58).

## Validation checklist

- **Evidence/preconditions:** PASS. WO-001 is `in-review`; the implementer,
  changed files, command attempts/results, and unavailable native smoke are
  recorded in the work order and execution log. The branch and working-tree
  diff provide meaningful comparison evidence.
- **Boundary/comparability:** PASS. The diff is limited to
  [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:23),
  the authorized requirements/desktop-blueprint wording, and recorded Forge
  evidence. No WO-002/003/004 implementation or unrelated core/persistence
  change is present. The untracked initiative directory contains the expected
  work-order/evidence artifacts; the changed-file record is consistent with
  the observed status and diff.
- **Typed shared width resolution:** PASS. `ToolbarWidthOverride` is attached
  to each typed item and `toolbar_item_width()` is reused by allocation and
  rendering. Elements resolves to the evidence-backed 92-point override;
  ordinary items remain measured with the 46-point fallback; Space remains 72
  points.
- **Elements width and containment:** PASS. Elements uses the fixed override,
  constrains the child UI and ComboBox to that width, retains truncation, sets
  the control response boundary, and has deterministic ledger assertions for
  response, child, group, and final-edge containment. The active role-label
  metric test demonstrates that the target permits the supported labels without
  broad truncation at the tested budget.
- **Geometry/routing/order:** PASS. Tests cover 4-point row padding, 4-point
  gaps, equal Flexible Space distribution, 72-point Space, actual content/clip
  intersection, leftmost-fit behavior, far-right `>>` overflow routing, and
  the approved New/Open/Save/Save As/Flexible Space/Elements/Flexible
  Space/Export PDF order.
- **Hover/label ownership:** PASS. Action click/hover remains the explicit
  icon-plus-padding response; the label is painted separately and is not made
  part of the response or tooltip. The control cursor intent is the standard
  arrow and Elements has an explicit control response. Deterministic tests
  cover response geometry and label-color behavior.
- **Durable-document classification:** PASS. The WO-001 `update/update`
  classification matches the actual user-facing width/hover contract: the
  requirements acceptance wording and desktop blueprint both receive narrow,
  evidence-backed updates. No semantic/core or persistence contract was
  changed.
- **Project-policy checks:** PASS. All applicable configured Cargo and diff
  checks passed. No required manual/native gate is configured.
- **Rogue behavior:** PASS. No deceptive, backdoor, malicious, nefarious, or
  otherwise unrelated behavior was observed.

## Findings

### Blockers

- None. No plainly unmet authorized criterion, unrelated implementation,
  required-policy failure, or comparison blocker was found.

### Warnings

- **WO001-VAL-W-001 — Native-window smoke unavailable.** **Severity:** warning.
  **Evidence:** The work order and execution log explicitly record that no
  window-capable native smoke was run. The manifest defines no native smoke as a
  required manual gate. **Recommendation:** Preserve the limitation and do
  not claim observed native hover rendering; retain native smoke as follow-up
  evidence if a window-capable run becomes available. **Required next state:**
  `complete`.

### Notes

- **WO001-VAL-N-001 — Diff is uncommitted.** **Severity:** note. **Evidence:**
  `git diff master...HEAD` has no committed implementation delta, while the
  working-tree `git diff` and status show the expected three implementation/
  durable-document files plus the untracked initiative evidence directory.
  **Recommendation:** Commit through the normal Forge closeout path; this does
  not prevent the current implementation comparison. **Required next state:**
  `complete`.
- **WO001-VAL-N-002 — Diagnostic source review.** **Severity:** note. The
  plausible failure sources considered were duplicated width resolution,
  intrinsic ComboBox expansion, clipped-edge budgeting, spacing/overflow
  accounting, broad hover ownership/label regression, durable-document drift,
  and missing policy evidence. The focused tests, source diff, durable-document
  diff, and policy command results reduce the likely risks to none requiring a
  fix; only native visual confirmation remains unavailable.
  **Recommendation:** No implementation change. **Required next state:**
  `complete`.

## Exact commands and results

| Command | Result |
|---|---|
| `git status --short --branch` | Exit 0; branch `dev/2026-10-27-toolbar-and-titlebar-polish`; modified requirements, blueprint, and `apps/scriptcraft/src/main.rs`; untracked initiative directory. |
| `git diff --stat` | Exit 0; 3 tracked files, 80 insertions, 19 deletions. |
| `git diff --check` | Exit 0; no whitespace errors. |
| `cargo test -p scriptcraft toolbar_` | Exit 0; 26 focused tests passed, 0 failed. |
| `cargo fmt --all -- --check` | Exit 0. |
| `cargo check --workspace` | Exit 0. |
| `cargo test --workspace` | Exit 0; 65 app tests and 44 core tests passed, with existing integration suites passing. |
| `cargo build --workspace` | Exit 0. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Exit 0. |
| `git diff master...HEAD` | Exit 0; no committed delta because implementation changes are uncommitted; working-tree diff was used for comparison. |

## Diff/file comparison

Observed tracked implementation changes are confined to:

- [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:23):
  46-point normal baseline reconciliation, typed width overrides, 92-point
  Elements and 72-point Space contracts, shared width use, control/ComboBox
  containment, standard-arrow toolbar control intent, and focused tests.
- [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:115):
  typed override behavior added to AC-WRITING-005.5.
- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:58):
  typed width-policy and shared resolver behavior added to the desktop
  presentation contract.

The recorded initiative plan, WO-001, and execution log are the expected
untracked Forge evidence files. No source changes for later work orders were
observed. The implementation boundary is comparable to the approved WO-001
boundary and the authorized durable-document classification matches the diff.

## Decision

- **Outcome:** `PASS WITH WARNINGS`
- **Required next state:** `complete`
- **Fix work order required:** `no`
- **WO-001 disposition:** WO-001 should become `complete`; no letter-suffixed
  fix work order is warranted. The native smoke limitation is a non-blocking
  warning under project policy and does not prevent meaningful comparison.
- **Implementation/design claim boundary:** This artifact validates the
  implemented WO-001 alignment only. It does not claim implementation,
  validation, or design approval for WO-002, WO-003, or WO-004.

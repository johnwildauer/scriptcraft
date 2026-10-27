# Validation: WO-001a — Reconcile Local Toolbar Geometry

## Validation metadata

- **Validation session/agent ID:** `forge-validator-toolbar-exact-width-hover-fix-2026-10-27-0742Z`
- **Validation type:** Fresh-context implementation validation
- **Date:** 2026-10-27
- **Work order:** [`WO-001a-reconcile-local-toolbar-geometry.md`](../work-orders/WO-001a-reconcile-local-toolbar-geometry.md:1)
- **Initiative:** [`toolbar-exact-width-hover-v1.md`](../toolbar-exact-width-hover-v1.md:1)
- **Original work order:** [`WO-001-exact-runtime-label-width.md`](../work-orders/WO-001-exact-runtime-label-width.md:1)
- **Failing validation:** [`validation-WO-001.md`](validation-WO-001.md:1)
- **Implementer session:** `code-implementer-toolbar-exact-width-hover-fix-2026-10-27-0740Z`
- **Session exception:** None

## Comparison basis

Reviewed [`00-overview.md`](../../../process/00-overview.md:1), [`05-validation.md`](../../../process/05-validation.md:1), [`project-manifest.md`](../../../project-manifest.md:1), the initiative plan, the failed [`validation-WO-001.md`](validation-WO-001.md:1), [`WO-001a-reconcile-local-toolbar-geometry.md`](../work-orders/WO-001a-reconcile-local-toolbar-geometry.md:1), the execution log, the implementer changed-file evidence, the current source, and the current workspace gates.

The meaningful fix comparison is the implementer's recorded WO-001a change: only four stale toolbar test assertion areas in [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1615) were updated. The working tree also contains the earlier WO-001 implementation and its authorized requirement/blueprint edits, so the complete uncommitted file diff is not treated as a WO-001a-only diff. The recorded changed-file evidence and source inspection provide the fix-scoped comparison.

## Diagnostic assessment

### Candidate sources considered

1. The four assertions could still expect the predecessor values identified by the failed validation.
2. The fix could have changed the production toolbar constants while updating those assertions.
3. The fix could have altered exact runtime text measurement or allocation propagation.
4. The fix could have altered icon hover ownership or hover-region geometry implementation.
5. The fix could have altered native viewport configuration.
6. The fix could have changed unrelated source or durable requirement/blueprint documents.
7. One or more required Cargo or diff gates could still fail despite the implementer report.

The two most likely sources were (a) stale assertions retaining predecessor geometry values and (b) an out-of-scope production edit hidden in the fix. The validation log is the fresh `cargo test --workspace` result plus the format, check, build, Clippy, and diff-gate results. No production logging was added because this validation is read-only with respect to implementation code.

## Findings

### PASS — Four stale assertions match the existing user-local production constants

**Severity:** Pass

**Evidence:** The current production constants in [`main.rs`](../../../../apps/scriptcraft/src/main.rs:28) are `TOOLBAR_TOP_PADDING = 32.0`, `TOOLBAR_BOTTOM_PADDING = 1.0`, and `TOOLBAR_ICON_HOVER_PADDING = 0.0`. The four stale assertion areas now match those values: the centered icon-region assertions at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1615) expect the zero-padding rectangle, the hover-fill test at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1629) expects zero icon padding, the manual geometry test at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1763) expects top `32.0` and bottom `1.0`, and the vertical-padding test at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1876) expects a total of `33.0`. The fresh workspace test run passed all 60 application tests, including these four tests.

**Recommendation:** None.

**Required next state:** Retain the reconciled test expectations.

### PASS — Production exact measurement, hover, and viewport code is unchanged by WO-001a

**Severity:** Pass

**Evidence:** The fix work order and execution log identify only test assertions as changed by WO-001a. Source inspection confirms the existing production exact measurement path remains in [`toolbar_label_width()`](../../../../apps/scriptcraft/src/main.rs:188), [`toolbar_item_width()`](../../../../apps/scriptcraft/src/main.rs:173), [`allocate_toolbar()`](../../../../apps/scriptcraft/src/main.rs:233), and [`toolbar()`](../../../../apps/scriptcraft/src/main.rs:592). The existing icon hover implementation remains in [`toolbar_action_button()`](../../../../apps/scriptcraft/src/main.rs:490) and [`toolbar_icon_hover_rect()`](../../../../apps/scriptcraft/src/main.rs:1443), with the existing viewport configuration in [`native_options()`](../../../../apps/scriptcraft/src/main.rs:1531). The production constants remain the pre-existing user-local values; no constant, exact-measurement, hover implementation, or viewport change is attributable to WO-001a. Requirements and blueprint files are unchanged by this fix according to the changed-file record.

**Recommendation:** None.

**Required next state:** Preserve the production implementation and durable documents.

### PASS — Required Cargo and diff gates

**Severity:** Pass

**Evidence:** Fresh commands all passed with exit code `0`: [`cargo fmt --all -- --check`](../../../../Cargo.toml:1), [`cargo check --workspace`](../../../../Cargo.toml:1), [`cargo test --workspace`](../../../../Cargo.toml:1), [`cargo build --workspace`](../../../../Cargo.toml:1), [`cargo clippy --workspace --all-targets -- -D warnings`](../../../../Cargo.toml:1), and [`git diff --check`](../../../../.gitignore:1). The workspace test gate reported 60 application tests, 44 core unit tests, integration suites of 2, 5, 4, 6, 2, 10, 5, and 3 tests, and 0 doc tests, with no failures.

**Recommendation:** None.

**Required next state:** The required automated gates are satisfied.

## Rogue-scope assessment

No deceptive, backdoor, malicious, nefarious, or otherwise rogue behavior was observed. No unrelated WO-001a source change was identified. Native visual smoke remains unavailable and is not a required project-policy gate; no pixel-level claim is made.

## Result

- **Outcome:** `pass`
- **Hard-stop finding:** None.
- **WO-001a status:** Remains `in-review` in this artifact; the user explicitly instructed that WO-001a not be marked complete.
- **WO-001 status:** Remains `in-review`; it must not be marked complete by this validation artifact.
- **Initiative next state:** `validate` evidence is recorded; the orchestrator may perform the permitted status transition separately, without changing production code or validating WO-002 here.
- **WO-002:** Not validated and remains pending.


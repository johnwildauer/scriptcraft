# Validation — WO-001 Vertical Edge Clamping

## Validation metadata

- **Validation status:** `PASS WITH WARNINGS`
- **Work order:** [`WO-001-vertical-edge-clamping.md`](../work-orders/WO-001-vertical-edge-clamping.md:1)
- **Initiative:** [`editor-geometry-corrections-v1.md`](../editor-geometry-corrections-v1.md:1)
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `debug-fresh-context-editor-geometry-corrections-WO-001-validation-2026-10-25T22:01:40Z`
- **Fresh-context statement:** This validation was performed as a separate fresh-context session. I inspected the validation and lifecycle rules, initiative, approved design review, WO-001, execution log, changed-file diff, and implementation/tests. I did not implement fixes, did not validate WO-002, and did not mark WO-001 complete.
- **Work-order state at validation:** `in-review`; WO-001 remains not complete.

## Comparison basis

The comparison boundary is only first/last visual-line edge clamping and its explicitly preserved adjacent behavior:

- Up on the first visual line must target that current line's `source.start`.
- Down on the last visual line must target that current line's `source.end`.
- Adjacent-line preferred horizontal goal behavior, UTF-8 boundaries, Shift selection, wrapped-line behavior, and preferred-goal reset semantics must remain correct.
- WO-002 screenplay width/leading behavior was not assessed.

Inspected:

- Validation rules in [`05-validation.md`](../../../process/05-validation.md:1) and lifecycle rules in [`00-overview.md`](../../../process/00-overview.md:1).
- Initiative scope and boundary in [`editor-geometry-corrections-v1.md`](../editor-geometry-corrections-v1.md:45), including the scoped approval for WO-001 at [`editor-geometry-corrections-v1.md`](../editor-geometry-corrections-v1.md:107).
- Approved design review, including the edge-clamp oracle warning, in [`design-review.md`](../reviews/design-review.md:51).
- WO-001 scope, acceptance criteria, changed-file record, and implementation evidence in [`WO-001-vertical-edge-clamping.md`](../work-orders/WO-001-vertical-edge-clamping.md:16).
- Dispatch, completion, and prior check evidence in [`execution-log.md`](../execution-log.md:8).
- The implementation in [`ScriptCraftApp::move_vertical()`](../../../../apps/scriptcraft/src/main.rs:671) and focused regressions in [`vertical_navigation_clamps_single_line_document_edges()`](../../../../apps/scriptcraft/src/main.rs:1072), [`vertical_navigation_clamps_wrapped_first_and_last_lines()`](../../../../apps/scriptcraft/src/main.rs:1110), and [`vertical_edge_clamps_reset_goal_repeat_safely_and_preserve_shift_anchor()`](../../../../apps/scriptcraft/src/main.rs:1109).

No clearly unrelated, deceptive, backdoor, malicious, or nefarious behavior was found. The diff is limited to [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1); WO-002 files and core layout/profile files were not changed by this implementation.

## Diagnostic assessment

Potential sources considered were: (1) missing same-line edge target selection; (2) wrong current visual-line lookup for wrapped first/last lines; (3) invalid UTF-8 endpoint arithmetic; (4) preferred horizontal goal being retained or recomputed across an edge clamp; (5) Shift replacing the selection anchor instead of moving only focus; (6) repeated edge presses becoming unsafe or drifting; and (7) unrelated layout/profile changes broadening the work order. The two most likely sources were the missing target branch and edge-reset behavior identified by the approved design review. Source comparison and focused tests validate that the new branch selects the current line's source endpoint, clears the preferred goal, and preserves the existing Shift anchor/focus path.

## Acceptance assessment

| WO-001 obligation | Result | Evidence |
|---|---|---|
| Up from a non-edge offset on the first visual line reaches `source.start` | **PASS** | [`move_vertical()`](../../../../apps/scriptcraft/src/main.rs:671) creates a same-line target using `current_line.source.start` when no preceding line exists. The single-line and wrapped-line regressions pass. |
| Down from a non-edge offset on the last visual line reaches `source.end` | **PASS** | [`move_vertical()`](../../../../apps/scriptcraft/src/main.rs:671) creates a same-line target using `current_line.source.end` when no following line exists. The single-line and wrapped-line regressions pass. |
| Wrapped-line behavior | **PASS** | [`vertical_navigation_clamps_wrapped_first_and_last_lines()`](../../../../apps/scriptcraft/src/main.rs:1110) builds a multi-line wrapped fixture and checks the actual first/last layout source ranges. |
| Adjacent-line preferred goal | **PASS** | The adjacent target path and preferred-column calculation remain unchanged in [`move_vertical()`](../../../../apps/scriptcraft/src/main.rs:717); the existing wrapped navigation regression continues to pass. |
| UTF-8 boundaries | **PASS** | The wrapped Unicode fixture uses emoji and asserts the final edge offset is a character boundary in [`vertical_navigation_clamps_wrapped_first_and_last_lines()`](../../../../apps/scriptcraft/src/main.rs:1110). Source endpoints are used directly rather than byte arithmetic. |
| Shift selection | **PASS** | The edge branch retains `selection.anchor` and changes only focus when `extend` is true; [`vertical_edge_clamps_reset_goal_repeat_safely_and_preserve_shift_anchor()`](../../../../apps/scriptcraft/src/main.rs:1109) verifies anchor preservation. |
| Reset semantics and repeated edge movement | **PASS** | The edge branch clears `preferred_vertical_x` before selecting the endpoint, and the focused regression verifies repeated presses remain at the edge with no preferred goal. |
| Scope and durable-document classification | **PASS** | The implementation diff changes only [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1), matching WO-001's transient app-navigation scope and `verify unchanged` requirements/blueprints classification. |

## Exact check results

Executed from the repository root on 2026-10-25:

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | **PASS**, exit 0 |
| `cargo check --workspace` | **PASS**, exit 0 |
| `cargo test --workspace` | **PASS**, exit 0 — 22 app tests, 28 core unit tests, 5 WO-003 integration tests, 3 WO-008 integration tests, 0 doc-tests; 58 tests passed, 0 failed |
| `cargo clippy --workspace --all-targets -- -D warnings` | **PASS**, exit 0 |
| Native smoke: `cargo run -p scriptcraft --quiet` | **WARNING / UNAVAILABLE FOR INTERACTION** — the native binary launched and remained running during the bounded attempt, but this validation context exposed no observable or interactable window. The process was stopped after the attempt. No native keyboard or visual edge-clamp result is claimed. Project policy identifies desktop manual checks as non-required, so this does not block validation. |

## Findings

### EGC-WO001-W-001 — Native visual edge-clamp smoke unavailable

- **Severity:** warning
- **Evidence:** [`cargo run -p scriptcraft --quiet`](../../../../Cargo.toml:1) launched [`target/debug/scriptcraft`](../../../../apps/scriptcraft/Cargo.toml:1) and remained running, but no interactable native window was available to place the caret and press Up/Down. Headless focused regressions and all required automated checks passed.
- **Recommendation:** Preserve the limitation and obtain native keyboard/visual evidence only in a capable desktop session; do not represent the automated tests as native proof.
- **Required next state:** Note only; no fix work order under the non-required manual-gate policy in [`project-manifest.md`](../../../project-manifest.md:88).

## Verdict and next state

- **Verdict:** `PASS WITH WARNINGS`.
- **Implementation alignment:** Pass. The implementation satisfies the authorized WO-001 edge-clamping behavior, preserves adjacent navigation, UTF-8-safe source endpoints, Shift selection, wrapped-line handling, and edge reset semantics. No genuine hard-stop, unrelated, deceptive, backdoor, malicious, or nefarious finding was identified.
- **Orchestrator action:** The orchestrator should complete WO-001 under the pass-with-warnings rule in [`05-validation.md`](../../../process/05-validation.md:79), link this validation from the execution record, and proceed to the separately gated WO-002 flow. It should not create a fix WO.
- **Status guard:** This validator did not mark WO-001 complete. WO-002 was not validated.

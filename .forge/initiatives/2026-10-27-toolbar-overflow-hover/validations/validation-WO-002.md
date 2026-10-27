# Validation — WO-002 Toolbar Group Hover and Color

## Validation metadata

- **Validation type:** `fresh-context implementation alignment and anti-rogue validation`
- **Status:** `FAIL`
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `debug-fresh-context-toolbar-overflow-hover-WO-002-validation-2026-10-27T0513Z`
- **Date:** 2026-10-27
- **Work order:** [`WO-002-toolbar-group-hover-and-color.md`](../work-orders/WO-002-toolbar-group-hover-and-color.md:1)
- **Initiative:** [`toolbar-overflow-hover-v1.md`](../toolbar-overflow-hover-v1.md:1)
- **Fresh-context statement:** This validation was performed as a separate fresh-context validator session. I did not implement WO-002, did not rely on implementer conversational memory, did not validate WO-001 from this artifact, and did not mark WO-002 complete.

## Scope and comparison basis

This validation covers only the complete outer Item/group hover and click surface, unchanged label text styling, centered layout and preserved semantics, `toolbar_chrome_color()` using `style.widgets.open.weak_bg_fill`, absence of global theme mutation, deterministic tests, authorized durable-document impact, and required Cargo/diff gates. It does not reassess WO-001 geometry or unimplemented future work.

Inspected:

- Validation/lifecycle rules in [`05-validation.md`](../../../process/05-validation.md:1) and [`00-overview.md`](../../../process/00-overview.md:1).
- Project commands, required gates, manual-gate policy, and durable-document rules in [`project-manifest.md`](../../../project-manifest.md:73).
- Initiative scope/order and native-evidence limitation in [`toolbar-overflow-hover-v1.md`](../toolbar-overflow-hover-v1.md:24), the fresh design review in [`design-review.md`](../reviews/design-review.md:1), and the prior WO-001 validation in [`validation-WO-001.md`](validation-WO-001.md:1).
- Implementer report, changed-file record, and gate limitation in [`WO-002-toolbar-group-hover-and-color.md`](../work-orders/WO-002-toolbar-group-hover-and-color.md:13) and [`execution-log.md`](../execution-log.md:57).
- Comparable working-tree diff for [`main.rs`](../../../../apps/scriptcraft/src/main.rs:446), the authorized requirements wording in [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:104), and the authorized desktop blueprint wording in [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:66).

## Diagnostic hypotheses and validation logs

### Candidate sources considered

1. The response might still cover only the icon control rather than the complete allocated Item/group rectangle.
2. The response might not own click dispatch, leaving the label area non-clickable.
3. The icon/label layout might no longer be centered within the final WO-001 group bounds.
4. Hover painting might alter label text color or otherwise mutate the shared/global theme.
5. `toolbar_chrome_color()` might still read `visuals.panel_fill`, or the helper might mutate the style while reading it.
6. Action ordering, overflow routing, role semantics, or accessibility tooltip text might have changed incidentally.
7. The deterministic tests or required project-policy gates might be absent or failing.

### Narrowed likely sources and logs

The two most likely implementation risks were (1) an incomplete outer response/click surface and (2) a required-gate failure hidden by the implementer evidence. Repository inspection and fresh command execution validate the first risk as aligned but confirm the second as a hard stop:

- **Outer-surface log:** [`toolbar_action_button()`](../../../../apps/scriptcraft/src/main.rs:446) allocates the final `width` and two interaction heights, calls `ui.interact(rect, id, Sense::click())`, applies the hover fill to that response's `rect`, and dispatches the action from the same response. The label is therefore inside the hover/click rectangle rather than a sibling outside it.
- **Centering/label log:** The scoped child UI uses `max_rect(rect)` with `Layout::top_down(Align::Center)` and renders the icon followed by `ui.label(item.label)`. No explicit label color override was added; the hover helper paints only the outer rectangle.
- **Color-source log:** [`toolbar_chrome_color()`](../../../../apps/scriptcraft/src/main.rs:1387) returns `ctx.style().visuals.widgets.open.weak_bg_fill`, and [`toolbar_frame()`](../../../../apps/scriptcraft/src/main.rs:1397) uses that value. No `set_style`, global visual assignment, or other theme mutation was introduced in the comparable diff.
- **Determinism log:** [`toolbar_chrome_uses_open_widget_fill_without_mutating_style()`](../../../../apps/scriptcraft/src/main.rs:1480) asserts the requested source and unchanged panel fill; [`toolbar_group_hover_response_is_deterministic_and_preserves_label_color()`](../../../../apps/scriptcraft/src/main.rs:1498) asserts transparent/non-hovered and hovered-widget-filled states plus stable text color.
- **Semantics/diff log:** The diff preserves existing action dispatch, Item ordering, overflow routing, Elements role selection, tooltip text, and WO-001 geometry. Changed durable documents are limited to the authorized requirements and desktop blueprint wording updates. No unrelated core, persistence, export, page-layout, deceptive, backdoor, malicious, or other rogue behavior was observed.
- **Gate log:** The exact required sequence stopped at [`cargo fmt --all -- --check`](../../Cargo.toml:1) because `main.rs` has formatter differences at lines 446 and 1486. Running the remaining checks independently showed `cargo check --workspace`, `cargo test --workspace` (54 app tests, 44 core tests, all listed integration suites, and 0 doc tests), and `cargo build --workspace` passed; `cargo clippy --workspace --all-targets -- -D warnings` failed on `clippy::obfuscated-if-else` in [`toolbar_group_fill()`](../../../../apps/scriptcraft/src/main.rs:1390). [`git diff --check`](../../../../.gitignore:1) passed.

## Findings

### N-001 — Full outer Item/group hover and click surface is aligned

- **Severity:** note
- **Evidence:** [`toolbar_action_button()`](../../../../apps/scriptcraft/src/main.rs:446) allocates the complete action group rectangle, obtains one outer `Response` with `Sense::click()`, paints the hover fill over that rectangle, and dispatches clicks from the same response.
- **Recommendation:** None for the implementation criterion.
- **Required next state:** Retain the outer response as the authorized hover/click surface.

### N-002 — Centered layout, stable label styling, and semantics are aligned

- **Severity:** note
- **Evidence:** The scoped UI centers the icon-first vertical content within the allocated group rectangle and renders the existing label without a hover color override. The diff preserves action dispatch, ordering, overflow routing, role behavior, tooltip text, and the prior WO-001 geometry.
- **Recommendation:** None for the implementation criterion.
- **Required next state:** Accept these WO-002 behavior claims subject to the required gates below.

### N-003 — Requested open-widget color source and no global theme mutation are aligned

- **Severity:** note
- **Evidence:** [`toolbar_chrome_color()`](../../../../apps/scriptcraft/src/main.rs:1387) reads `style.visuals.widgets.open.weak_bg_fill` through the context style, and the deterministic test verifies the frame fill and unchanged `panel_fill`. No global style mutation appears in the diff.
- **Recommendation:** None for the implementation criterion.
- **Required next state:** Retain the authorized local style-source behavior.

### N-004 — Authorized deterministic tests and durable-document impact are aligned

- **Severity:** note
- **Evidence:** The two added tests cover open-widget source/non-mutation and deterministic hover fill/stable label color. The diff updates only the requirements and desktop blueprint passages authorized by the work order's `update` classifications; no unrelated durable document was changed.
- **Recommendation:** Retain the requirements/blueprint updates and their traceability.
- **Required next state:** Treat the durable-document classification as verified.

### H-001 — Required format gate failed

- **Severity:** hard stop
- **Evidence:** Fresh [`cargo fmt --all -- --check`](../../Cargo.toml:1) exited nonzero and reported formatter changes required in [`main.rs`](../../../../apps/scriptcraft/src/main.rs:446) and the style-source test around line 1486. The implementer explicitly did not run fresh validation, so this is a new validator result.
- **Recommendation:** Dispatch a scoped fix work order or otherwise apply the project's authorized formatting remediation, then rerun the complete required sequence. This validator did not modify production code.
- **Required next state:** Keep WO-002 `in-review`; do not mark it complete until the required format gate passes.

### H-002 — Required Clippy gate failed

- **Severity:** hard stop
- **Evidence:** Fresh [`cargo clippy --workspace --all-targets -- -D warnings`](../../Cargo.toml:1) exited nonzero on `clippy::obfuscated-if-else` in [`toolbar_group_fill()`](../../../../apps/scriptcraft/src/main.rs:1390). The command suggested replacing the `then_some(...).unwrap_or(...)` chain with an `if`/`else` expression.
- **Recommendation:** Dispatch a scoped fix work order for the Clippy violation, then rerun all required gates. Before any implementation fix is dispatched, the diagnosis must be confirmed by the orchestrator/user; this validation session made no fix.
- **Required next state:** Keep WO-002 `in-review`; a concrete fix work order is required because a required project-policy gate failed.

### W-001 — Native visual confirmation is unavailable

- **Severity:** warning
- **Evidence:** The execution log and work-order evidence state that no visible desktop test surface is available. This fresh validation session likewise had no native interaction channel.
- **Recommendation:** Preserve this as an honest manual-evidence limitation. Do not claim native hover, color, pixel, or OS-chrome success. The manifest configures no manual/external hard-stop gate, so this warning is not the cause of failure.
- **Required next state:** Note/follow-up only; no native visual claim.

## Required check results

The validator attempted the exact required sequence:

```text
cargo fmt --all -- --check && cargo check --workspace && cargo test --workspace && cargo build --workspace && cargo clippy --workspace --all-targets -- -D warnings && git diff --check
```

| Check | Fresh result | Outcome |
|---|---|---|
| `cargo fmt --all -- --check` | Exit 1; formatter differences reported in `main.rs` | FAIL |
| `cargo check --workspace` | Exit 0 | PASS |
| `cargo test --workspace` | Exit 0; app 54 passed, core 44 passed, all integration suites passed, doc tests 0 | PASS |
| `cargo build --workspace` | Exit 0 | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | Exit 1; `clippy::obfuscated-if-else` in `toolbar_group_fill()` | FAIL |
| `git diff --check` | Exit 0; no whitespace errors | PASS |

The exact chained sequence stopped at the first formatting failure. The later checks were rerun independently to provide complete evidence; no formatter command was run in write mode.

## Verdict and required next state

- **Verdict:** `FAIL`.
- **Why:** The implementation aligns with the requested outer group hover/click surface, stable label styling, centered layout, preserved semantics, requested open-widget color source, no global theme mutation, deterministic tests, and authorized durable-document impact. However, two required project-policy gates fail: formatting and Clippy with `-D warnings`.
- **Required next state:** Keep [`WO-002-toolbar-group-hover-and-color.md`](../work-orders/WO-002-toolbar-group-hover-and-color.md:1) `in-review`. The orchestrator should record this validation and create a letter-suffixed fix work order for the two concrete gate failures, after confirming the diagnosis before implementation. The fix must receive a new implementer session and a fresh follow-up validation; WO-002 must not be marked complete from this artifact.
- **Scope guard:** This artifact does not modify production code, does not mark WO-002 complete, does not claim native visual success, and does not validate future scope.

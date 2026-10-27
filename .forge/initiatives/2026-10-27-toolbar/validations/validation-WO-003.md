# Validation: WO-003 — Toolbar Spacing and Overflow

## Validation metadata

- **Review type:** `validation`
- **Status:** `in-review`
- **Validator:** Fresh-context Forge debug validator
- **Validator session/agent ID:** `debug-fresh-context-toolbar-WO-003-validation-2026-10-27T02:44:47Z`
- **Session exception:** `none`
- **Date:** `2026-10-27`
- **Initiative:** [`toolbar-v1.md`](../toolbar-v1.md:1)
- **Validation file:** `validation-WO-003.md`
- **Validated work order:** [`WO-003-toolbar-spacing-and-overflow.md`](../work-orders/WO-003-toolbar-spacing-and-overflow.md:1)
- **Compared commit/diff:** Working-tree implementation diff; no commit was available. The compared boundary was [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1), the authorized requirements update at [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:113), the authorized desktop blueprint update at [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:68), and the WO-003 implementation/evidence artifacts.
- **Fix chain:** `none`

## Fresh-context statement and authorized scope

This validation was performed in a separate fresh context. I read the Forge
overview and validation process, project manifest, toolbar initiative, design
review, WO-003, the WO-001 and WO-002 implementation/validation evidence,
execution log, validation template, changed-file record, implementation source,
focused tests, durable-document updates, and working-tree diff. I did not
implement changes, did not mark WO-003 complete, and did not validate unrelated
work.

The assessment is limited to WO-003: deterministic normal/Space width, equal
Flexible Space allocation, leftmost-fit narrow behavior, the far-right `>>`
overflow affordance, executable action entries, nested Elements menu routing,
focused regressions, authorized durable-document updates, and the required
Cargo gates. Native visual success is not claimed because interactive window
inspection was unavailable.

## Comparison basis and checklist

- The work order is `in-review`, and the execution log identifies the separate
  implementer session [`code-implementer-toolbar-wo003-2026-10-27-0241`](../work-orders/WO-003-toolbar-spacing-and-overflow.md:13).
- The changed-file record and bounded diff are consistent: the production
  behavior is in the desktop app, requirements and desktop-blueprint updates
  are the authorized durable-document changes, and the initiative evidence
  artifacts are present. No core semantic model, persistence schema, page
  layout, export implementation, or project serialization was changed by the
  recorded WO-003 scope.
- The pure transient allocator at [`allocate_toolbar()`](../../../../apps/scriptcraft/src/main.rs:166) uses the fixed [`TOOLBAR_NORMAL_ITEM_WIDTH`](../../../../apps/scriptcraft/src/main.rs:20) for every normal item, including [`ToolbarItemKind::Space`](../../../../apps/scriptcraft/src/main.rs:64), and does not mutate project state.
- When all normal items fit, [`allocate_toolbar()`](../../../../apps/scriptcraft/src/main.rs:166) divides the remaining width equally across each [`ToolbarItemKind::FlexibleSpace`](../../../../apps/scriptcraft/src/main.rs:64). The focused wide/exact-fit regression asserts both flexible widths at 136 points for an 800-point toolbar at [`toolbar_allocation_wide_and_exact_fit_are_deterministic()`](../../../../apps/scriptcraft/src/main.rs:1430), and the isolated two-flexible-space case asserts equal 118-point allocations at [`toolbar_allocation_two_flexible_spaces_split_remaining_width_equally()`](../../../../apps/scriptcraft/src/main.rs:1462).
- Narrow allocation reserves [`TOOLBAR_OVERFLOW_WIDTH`](../../../../apps/scriptcraft/src/main.rs:21), retains fitting non-flexible items in source order, excludes spacing items from the overflow list, and appends later non-spacing items to overflow. The 300-point regression retains New/Open/Save and extracts Save As/Elements/Export PDF in order at [`toolbar_allocation_narrow_keeps_leftmost_items_and_far_right_menu()`](../../../../apps/scriptcraft/src/main.rs:1448).
- Rendering places the overflow control after the visible sequence using the right-to-left layout and [`toolbar_overflow_menu()`](../../../../apps/scriptcraft/src/main.rs:451), with the literal `>>` affordance at [`ui.menu_button()`](../../../../apps/scriptcraft/src/main.rs:454). Action entries call [`dispatch_toolbar_action()`](../../../../apps/scriptcraft/src/main.rs:382), while an overflow Elements entry opens a nested menu and routes a selected role through [`change_role_from_toolbar()`](../../../../apps/scriptcraft/src/main.rs:431). The focused routing regression is [`overflow_entries_route_actions_and_elements_submenu_choices()`](../../../../apps/scriptcraft/src/main.rs:1477).
- The authorized requirements and desktop blueprint additions document the
  transient allocation, non-spacing overflow, far-right menu, action routing,
  and nested Elements contract at [`AC-WRITING-005.5`](../../../requirements/features/screenplay-writing-loop.md:113) and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:68). No unauthorized durable-document update was observed.
- No deceptive, backdoor, malicious, nefarious, or otherwise rogue behavior
  was observed in the compared changes.

## Independently rerun checks

The following project-policy commands were rerun in this fresh validation
session and passed:

- `cargo fmt --all -- --check` — PASS
- `cargo check --workspace` — PASS
- `cargo test --workspace` — PASS; 42 app tests, 44 core unit tests, all
  workspace integration/doc tests passed with zero failures. The focused WO-003
  allocation and routing regressions passed.
- `cargo build --workspace` — PASS
- `cargo clippy --workspace --all-targets -- -D warnings` — PASS
- `git diff --check` — PASS

A bounded `cargo run -p scriptcraft` launch started the binary and remained
running for the eight-second smoke window, but no observable native window
interaction was available. The process was terminated after that bounded
attempt. Native width changes, menu opening, action activation, nested role
selection, and visual placement are therefore not claimed as manually passed.

## Findings

### Blockers

- **None.** The comparison was meaningful, the implementation is within the
  authorized WO-003 boundary, the explicit deterministic allocation and
  overflow criteria are addressed by source and focused headless regressions,
  no rogue behavior was observed, and no required project-policy Cargo gate
  failed.

### Warnings

- **WO003-VAL-W-001 — Native visual interaction evidence unavailable. Severity:** warning. **Evidence:** The bounded [`cargo run -p scriptcraft`](../../../../Cargo.toml:1) launch started successfully but exposed no observable interactive window during the validation session. The work order defines native smoke as bounded/manual evidence and explicitly permits recording its unavailability at [`WO-003-toolbar-spacing-and-overflow.md`](../work-orders/WO-003-toolbar-spacing-and-overflow.md:60). **Recommendation:** Preserve this as an honest optional/manual evidence limitation; do not infer native visual success from source inspection or headless tests. **Required next state:** `complete` after the orchestrator records this warning; no fix work order is required because the manifest configures no manual/external hard-stop gate.

### Notes

- **WO003-VAL-N-001 — Deterministic spacing contract aligned. Severity:** note. **Evidence:** [`allocate_toolbar()`](../../../../apps/scriptcraft/src/main.rs:166) assigns one fixed normal slot, gives equal remaining allocation to flexible spaces, and the wide/exact-fit and two-flexible-space regressions pass. **Recommendation:** Accept the implementation as aligned for WO-003. **Required next state:** `complete`.
- **WO003-VAL-N-002 — Leftmost-fit and overflow contract aligned. Severity:** note. **Evidence:** The narrow regression at [`toolbar_allocation_narrow_keeps_leftmost_items_and_far_right_menu()`](../../../../apps/scriptcraft/src/main.rs:1448) verifies visible/overflow order and the fixed overflow affordance width; [`toolbar()`](../../../../apps/scriptcraft/src/main.rs:489) renders the affordance after visible items. **Recommendation:** Accept as aligned; native placement remains covered only by the warning above. **Required next state:** `complete`.
- **WO003-VAL-N-003 — Executable action and nested Elements routing aligned headlessly. Severity:** note. **Evidence:** [`toolbar_overflow_menu()`](../../../../apps/scriptcraft/src/main.rs:451) provides text action entries and a nested Elements submenu, and [`overflow_entries_route_actions_and_elements_submenu_choices()`](../../../../apps/scriptcraft/src/main.rs:1477) verifies the extracted Elements entry plus action/session adapters without a window. **Recommendation:** Accept the deterministic routing evidence without upgrading it to native visual evidence. **Required next state:** `complete`.
- **WO003-VAL-N-004 — Durable-document classification is accurate. Severity:** note. **Evidence:** The diff adds only the authorized responsive-toolbar acceptance criterion at [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:113) and transient desktop allocation/overflow contract at [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:68); toolbar state remains non-persistent and semantic role ownership remains in the core. **Recommendation:** Retain `Requirements impact: update` and `Blueprint impact: update`. **Required next state:** `complete`.

## Decision

- **Outcome:** `pass`
- **Required next state:** The orchestrator may transition WO-003 from `in-review` to `complete` after linking this validation result in the execution record. Preserve the native visual evidence warning, do not claim native visual success, and do not mark WO-003 complete in this validator session.
- **Fix work order required:** `no`

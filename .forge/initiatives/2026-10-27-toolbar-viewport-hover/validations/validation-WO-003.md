# Validation: WO-003 — Final Toolbar Edge Budget

## Validation metadata

- **Validation type:** `fresh-context implementation validation`
- **Validator session/agent ID:** `validation-WO-003-toolbar-viewport-hover-2026-10-27T0557Z`
- **Date:** 2026-10-27
- **Work order:** [`WO-003-final-toolbar-edge-budget.md`](../work-orders/WO-003-final-toolbar-edge-budget.md:1)
- **Initiative:** [`toolbar-viewport-hover-v1.md`](../toolbar-viewport-hover-v1.md:1)
- **Work-order status at validation:** `in-review`
- **Session separation exception:** none

## Comparison basis and authorized boundary

This fresh-context validation used [`00-overview.md`](../../../process/00-overview.md:1), [`05-validation.md`](../../../process/05-validation.md:1), [`project-manifest.md`](../../../project-manifest.md:1), the initiative plan, the fresh design review, the WO-003 implementation evidence, the WO-001 and WO-002 implementation/validation evidence, the execution log, the current source, the comparable diff, and a fresh required-gate run.

The comparison was limited to actual `TopBottomPanel`/content/clip right-edge reconciliation, shared allocation/rendering row bounds, final-label containment, wide/maximized and exact-fit behavior, narrow overflow, preservation of Flexible Space and existing semantics, deterministic tests, durable-document impact, and required Cargo/diff gates. No production code was modified by this validation.

WO-001 and WO-002 were treated as already validated dependencies. Their native visual-smoke limitation remains applicable; this validation does not claim native visual success.

## Diagnostic sweep

The independent comparison considered these possible sources of an edge-budget failure:

1. The allocator might still use a logical or estimated width instead of the actual visible panel/clip extent.
2. The renderer might allocate a different row width from the allocator, causing the final group to escape the shared budget.
3. Panel, frame, viewport, or clip insets might be omitted or applied twice at the right edge.
4. The final label might be excluded from the measured group width even when the Item itself fits.
5. Flexible Space might consume the wrong remainder or lose equal division after the budget correction.
6. Narrow overflow might lose its reserved `>>` width, reorder leftmost fitting Items, or expose spacing Items incorrectly.
7. The change might alter toolbar semantics, role routing, persistence behavior, durable-document alignment, or required gate status outside WO-003.

The two most likely risks were (1)/(2), because the work order specifically targets a discrepancy between the logical estimate and actual clipped content, and (4), because the acceptance criterion names the final label. Source inspection and the fresh deterministic test/gate run were used to validate those assumptions rather than infer success from the implementation report.

## Findings

### PASS — actual TopBottomPanel/content/clip edge reconciliation

- **Severity:** pass
- **Evidence:** [`toolbar_actual_available_width()`](../../../../apps/scriptcraft/src/main.rs:197) intersects the available panel content rectangle with [`Ui::clip_rect()`](../../../../apps/scriptcraft/src/main.rs:570), then budgets from the current cursor left edge to the clipped right edge. [`ScriptCraftApp::toolbar()`](../../../../apps/scriptcraft/src/main.rs:563) uses that result for [`allocate_toolbar()`](../../../../apps/scriptcraft/src/main.rs:215).
- **Assessment:** The implementation addresses the authorized actual-content/clip reconciliation rather than merely changing a fixed estimate. The test [`toolbar_shared_row_edge_contains_final_group_at_actual_content_edge()`](../../../../apps/scriptcraft/src/main.rs:1750) covers a content right edge beyond a narrower clip right edge and verifies the resulting 792-point available width.
- **Recommendation / required next state:** Retain the implementation; no fix work order is required.

### PASS — shared row bounds and final label containment

- **Severity:** pass
- **Evidence:** [`ToolbarAllocation::row_width`](../../../../apps/scriptcraft/src/main.rs:161) is derived from the same [`toolbar_content_width()`](../../../../apps/scriptcraft/src/main.rs:185) calculation used by allocation. [`ScriptCraftApp::toolbar()`](../../../../apps/scriptcraft/src/main.rs:575) passes that row width directly to the renderer, which includes row padding, inter-item gaps, visible groups, overflow reservation, and the final row padding. [`toolbar_row_right_edge()`](../../../../apps/scriptcraft/src/main.rs:207) and the actual-edge test assert the rendered allocation does not exceed the clip right edge and retains `Export PDF` as the final visible Item at the tested wide edge.
- **Assessment:** Allocator and renderer share one row budget. Normal Item widths use label-aware [`toolbar_item_width()`](../../../../apps/scriptcraft/src/main.rs:172), so the final label is included in the containment calculation. The source/test evidence supports the acceptance criterion that a fully fitting final Item, including its label, remains within the actual clipped edge.
- **Recommendation / required next state:** No edge-budget remediation is required.

### PASS — wide/maximized and exact-fit allocations

- **Severity:** pass
- **Evidence:** [`toolbar_shared_row_edge_contains_wide_and_exact_fit_allocations()`](../../../../apps/scriptcraft/src/main.rs:1766) checks a wide content rectangle and an exact width calculated from the complete default composition. It asserts right-edge containment and no overflow for the exact-fit case. The existing [`toolbar_allocation_wide_and_exact_fit_are_deterministic()`](../../../../apps/scriptcraft/src/main.rs:1659) and allocation-bound tests also passed in the fresh workspace run.
- **Assessment:** Wide/maximized-equivalent and exact-fit paths remain within their supplied actual row bounds without converting a fitting composition into overflow.
- **Recommendation / required next state:** Preserve the deterministic allocation behavior.

### PASS — narrow overflow, Flexible Space, and preserved semantics

- **Severity:** pass
- **Evidence:** [`toolbar_shared_row_edge_preserves_narrow_overflow_reservation()`](../../../../apps/scriptcraft/src/main.rs:1789) verifies the 32-point overflow reservation, the leftmost visible sequence, and the overflow sequence `Save As`, `Elements`, `Export PDF`. [`allocate_toolbar()`](../../../../apps/scriptcraft/src/main.rs:215) excludes Flexible Space from narrow overflow and divides the remaining width equally when the full composition fits. The existing [`toolbar_allocation_two_flexible_spaces_split_remaining_width_equally()`](../../../../apps/scriptcraft/src/main.rs:1817), default-order, overflow-dispatch, and role-choice tests passed unchanged.
- **Assessment:** Flexible Space remains typed and preserved in the wide path; narrow overflow retains leftmost-fit and far-right-menu semantics. No new command, role-policy, persistence, export, core, viewport, hover, or unrelated UI behavior was found in the WO-003 production change.
- **Recommendation / required next state:** No semantic or overflow fix is required.

### PASS — deterministic tests and durable-document impact

- **Severity:** pass
- **Evidence:** The fresh run passed the app's 60 unit tests, including the three new shared-edge tests, and the workspace's 44 core unit tests plus integration targets. The authorized update to [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:114) records the actual clipped edge and final-label containment contract. The authorized update to [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:72) records the shared clipped `TopBottomPanel` budget and inset reconciliation.
- **Assessment:** The declared `Requirements impact: update` and `Blueprint impact: update` in [`WO-003-final-toolbar-edge-budget.md`](../work-orders/WO-003-final-toolbar-edge-budget.md:41) match the user-facing allocation contract and the durable technical behavior. The documentation changes are within the approved WO-003 boundary.
- **Recommendation / required next state:** No documentation correction is required.

### PASS — required Cargo and diff gates

- **Severity:** pass
- **Evidence:** A fresh chained run completed with exit code 0 for `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`, `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check`. The run passed all 60 app tests, 44 core unit tests, all integration targets, and the named edge-budget tests. No whitespace errors were reported.
- **Assessment:** All required project-policy gates available in this checkout passed. The comparable diff contains the authorized [`main.rs`](../../../../apps/scriptcraft/src/main.rs:193) implementation and the authorized requirements/blueprint updates, along with initiative evidence files. No clearly unrelated, deceptive, backdoor, malicious, nefarious, or rogue behavior was found.
- **Recommendation / required next state:** No gate remediation is required.

### WARNING — native visual smoke unavailable

- **Severity:** warning (non-blocking under current project policy)
- **Evidence:** The WO-003 evidence and execution log record that no fresh native-window smoke was available. The manifest defines no required manual/external gate, and the prior WO-001/WO-002 validations explicitly preserve the same limitation.
- **Assessment:** This validation confirms source/configuration behavior and deterministic tests only. It does not claim native visual edge containment, native hover success, native transparent-titlebar success, or pixel-level maximized-window success.
- **Recommendation / required next state:** Preserve the limitation in the validation and work-order evidence. Do not convert it into a native visual success claim.

## Result

- **Outcome:** `pass with warning`
- **Hard-stop findings:** none
- **Warnings/notes:** Native visual smoke was unavailable; no native visual success is claimed.
- **WO-003 disposition:** Remains `in-review`; this validation does not mark WO-003 `complete`.
- **Scope boundary:** Only the final toolbar edge budget and its authorized durable-document/gate evidence were assessed. No production code was changed.

## Required next state

The orchestrator should record this validation in the initiative execution log and apply the normal lifecycle transition. WO-003 may advance only through the orchestrator-owned status transition after this `pass with warning`. Preserve the native visual-smoke limitation and do not claim native visual success without an available native smoke result.

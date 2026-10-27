# Validation: WO-002 — Uniform Persistent Icon Hover

## Validation metadata

- **Validation session/agent ID:** `forge-validator-toolbar-exact-width-hover-2026-10-27-0746Z`
- **Validation type:** Fresh-context implementation validation
- **Date:** 2026-10-27
- **Work order:** [`WO-002-uniform-persistent-icon-hover.md`](../work-orders/WO-002-uniform-persistent-icon-hover.md:1)
- **Initiative:** [`toolbar-exact-width-hover-v1.md`](../toolbar-exact-width-hover-v1.md:1)
- **Implementer session:** `code-implementer-toolbar-exact-width-hover-2026-10-27-0743Z`
- **Session exception:** None
- **Production-code modification by validator:** None

## Comparison basis

Reviewed the Forge process in [`00-overview.md`](../../../process/00-overview.md:1) and [`05-validation.md`](../../../process/05-validation.md:1), project policy in [`project-manifest.md`](../../../project-manifest.md:1), the initiative plan and design review, [`WO-002-uniform-persistent-icon-hover.md`](../work-orders/WO-002-uniform-persistent-icon-hover.md:1), the [`WO-001-exact-runtime-label-width.md`](../work-orders/WO-001-exact-runtime-label-width.md:1) and [`WO-001a-reconcile-local-toolbar-geometry.md`](../work-orders/WO-001a-reconcile-local-toolbar-geometry.md:1) implementation records, [`validation-WO-001.md`](validation-WO-001.md:1), [`validation-WO-001a.md`](validation-WO-001a.md:1), the initiative [`execution-log.md`](../execution-log.md:1), the implementer's changed-file/gate evidence, the current source, durable-document diff, and a fresh workspace gate run.

The implementation surface is [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1). The authorized durable-document surface is [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:90) and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:58). The working tree contains the earlier WO-001/WO-001a changes, so the full uncommitted file diff is not treated as a WO-002-only diff; the recorded changed-file evidence and source inspection provide the scoped comparison.

The comparison is meaningful: WO-002 is `in-review`, the implementer recorded changed files and gate attempts, and comparable source/diff evidence is present. WO-001 and WO-001a are prior validated dependencies and were not revalidated here except where needed to ensure WO-002 did not disturb their behavior.

## Diagnostic assessment

### Candidate sources considered

1. The hover rectangle could be expanded from a corner or top edge rather than centered on the icon control.
2. Horizontal and vertical padding could differ, or the named padding could be applied only on one axis.
3. A label widget, icon widget, or second response could compete with the intended hover/click owner.
4. Painter placement could leave the label inside the response or alter the label's normal styling.
5. The old visible-label tooltip could remain attached through another toolbar path.
6. Click dispatch or toolbar action metadata could be lost while changing the response ownership.
7. The deterministic tests or required Cargo/diff gates could pass in the implementer report but fail in the current workspace.

The two most likely sources were (a) incorrect response geometry/ownership in the action-button renderer and (b) an incomplete removal of tooltip or action-dispatch behavior. Source inspection and the fresh deterministic test/gate run validate both assumptions. No production logging was added because this is a read-only validation; the test output and source-level assertions are the diagnostic evidence. No diagnosis confirmation is required before fixing because this validation found no implementation hard stop and does not dispatch a fix.

## Findings

### PASS — Uniform centered icon hover geometry and equal padding

**Severity:** Pass

**Evidence:** [`toolbar_action_button()`](../../../../apps/scriptcraft/src/main.rs:490) derives [`icon_control_rect()`](../../../../apps/scriptcraft/src/main.rs:1466) from the group center and passes it to [`toolbar_icon_hover_rect()`](../../../../apps/scriptcraft/src/main.rs:1476). The helper uses [`Rect::from_center_size()`](../../../../apps/scriptcraft/src/main.rs:1477) and adds [`Vec2::splat()`](../../../../apps/scriptcraft/src/main.rs:1479) padding on both axes, so the hover bounds remain centered with identical left/right and top/bottom expansion. The deterministic geometry test at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1661) asserts all four edges equal the named padding; the centered/excluded-label test at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1646) asserts horizontal centering and that the label area is outside the response. The current named local padding is `0.0` at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:30), and the tests correctly validate the resulting control-sized region plus the helper's nonzero-padding behavior.

**Recommendation:** None for WO-002.

**Required next state:** Retain the centered equal-padding helper and tests.

### PASS — One explicit hover/click owner and decorative child non-competition

**Severity:** Pass

**Evidence:** The action path makes one explicit [`Ui::interact()`](../../../../apps/scriptcraft/src/main.rs:496) call on `icon_rect`, applies the cursor and hover fill to that response, and dispatches clicks from the same response at [`response.clicked()`](../../../../apps/scriptcraft/src/main.rs:535). The icon and label are rendered with [`Painter::text()`](../../../../apps/scriptcraft/src/main.rs:521) and [`Painter::text()`](../../../../apps/scriptcraft/src/main.rs:528), rather than child widgets that register competing responses. The fresh tests [`toolbar_icon_hover_owner_has_deterministic_outside_edge_and_center_positions()`](../../../../apps/scriptcraft/src/main.rs:1661) and [`toolbar_icon_hover_region_is_centered_and_excludes_the_label_area()`](../../../../apps/scriptcraft/src/main.rs:1646) pass.

**Recommendation:** None.

**Required next state:** Preserve one response owner and painter-only icon/label content in the action path.

### PASS — Label exclusion, normal styling, and redundant tooltip removal

**Severity:** Pass

**Evidence:** The label is painted at [`label_center`](../../../../apps/scriptcraft/src/main.rs:517), below the icon control, and uses [`ui.visuals().text_color()`](../../../../apps/scriptcraft/src/main.rs:533), with no hover-specific label color. The response is limited to `icon_rect`, and the source contains no [`on_hover_text()`](../../../../apps/scriptcraft/src/main.rs:497) call in the toolbar action path. The requirement now records that the visible label remains available for accessibility without a redundant tooltip at [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:104), and the desktop blueprint records the same interaction contract at [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:66).

**Recommendation:** None.

**Required next state:** Retain the label outside the response, normal text styling, and tooltip-free visible-label contract.

### PASS — Preserved semantics and click dispatch

**Severity:** Pass

**Evidence:** [`response.clicked()`](../../../../apps/scriptcraft/src/main.rs:535) continues to match [`ToolbarItemKind::Action`](../../../../apps/scriptcraft/src/main.rs:536) and calls [`dispatch_toolbar_action()`](../../../../apps/scriptcraft/src/main.rs:537). The dispatcher retains the existing New/Open/Save/Save As/Export PDF adapter routing at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:464). The action metadata test [`toolbar_action_metadata_keeps_icon_and_label_separate()`](../../../../apps/scriptcraft/src/main.rs:1578) and the full workspace test run passed; no core, persistence, export, role, or command implementation was changed by the WO-002 evidence.

**Recommendation:** None.

**Required next state:** Preserve the existing action adapter and semantic command routing.

### PASS — Durable-document impact

**Severity:** Pass

**Evidence:** WO-002 classified both Requirements and Blueprint impact as `update`. The requirements change at [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:104) removes the redundant visible-label tooltip while preserving the label/accessibility contract. The blueprint change at [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:66) makes the single response owner, separate label, normal label color, and no redundant tooltip explicit. These updates are directly subordinate to the approved user-facing interaction contract and match the source diff. No additional durable document impact was identified.

**Recommendation:** None.

**Required next state:** Retain the recorded Requirements/Blueprint updates; no documentation correction is required.

### PASS — Required Cargo and diff gates

**Severity:** Pass

**Evidence:** A fresh sequential run at validation time returned exit code `0` for [`cargo fmt --all -- --check`](../../../../Cargo.toml:1), [`cargo check --workspace`](../../../../Cargo.toml:1), [`cargo test --workspace`](../../../../Cargo.toml:1), [`cargo build --workspace`](../../../../Cargo.toml:1), [`cargo clippy --workspace --all-targets -- -D warnings`](../../../../Cargo.toml:1), and [`git diff --check`](../../../../.gitignore:1). The workspace test run passed 61 application tests, 44 core unit tests, integration suites of 2, 5, 4, 6, 2, 10, 5, and 3 tests, and 0 doc tests. The current status/diff inspection showed only the initiative's source, authorized requirements/blueprint updates, and initiative artifacts; no unrelated production file was identified.

**Recommendation:** None.

**Required next state:** Automated validation gates are satisfied.

## Native visual limitation

No native visual/window smoke was performed. The project policy records no required manual/external gate, and both the implementer evidence and prior validation evidence state that visible-window measurement is unavailable. This validation therefore makes no claim of native pixel-level hover success; the result is based on source inspection and deterministic geometry/behavior tests.

## Rogue-scope assessment

No deceptive, backdoor, malicious, nefarious, or otherwise rogue behavior was observed. The changed behavior is within the approved WO-002 toolbar interaction boundary. Earlier WO-001/WO-001a changes remain present as prior initiative work and were not silently attributed to WO-002.

## Result

- **Outcome:** `pass`
- **Hard-stop finding:** None.
- **WO-002 status:** Remains `in-review`; this validator does not mark work orders complete.
- **Initiative next state:** `validate` evidence is recorded; the orchestrator may perform the permitted status transition separately.
- **Required next state:** WO-002 may transition to `complete` under the Forge process after the orchestrator records this pass; no fix work order is required.
- **Native visual status:** Unavailable and not claimed.


# Work Order: WO-001 — Toolbar Geometry, Hover, and Elements Width

## Metadata

- **Status:** `complete`
- **Assignee:** Desktop toolbar implementer (one fresh session)
- **Phase/order:** 1 of 4; first implementation order
- **Initiative:** [`../toolbar-and-titlebar-polish-v1.md`](../toolbar-and-titlebar-polish-v1.md:1)
- **Generated filename:** `WO-001-toolbar-geometry-hover-and-elements-width.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-001.md`](../validations/validation-WO-001.md)
- **Fixes:** none
- **Implementer session/agent ID:** `implementer-wo001-toolbar-geometry-2026-10-27T1552Z`
- **Session exception:** none

## Summary

Make the toolbar a compact, predictable interaction surface and establish one resolved width contract for allocation and rendering. The outcome is a toolbar whose action hover response is limited to the icon control, whose labels remain separate, and whose Elements control receives an evidence-based per-item width without violating the actual clipped row edge.

## In scope

- Reconcile the current `TOOLBAR_NORMAL_ITEM_WIDTH`/test mismatch from the source snapshot instead of assuming either value is authoritative.
- Replace broad or boxy hover ownership with the smallest egui response/padding primitives compatible with the existing icon-first action renderer and accessibility labels.
- Add an explicit per-item width override or equivalent typed width policy, with a documented fallback for ordinary Items.
- Apply the same resolved width to `allocate_toolbar()`, action rendering, Elements rendering, overflow fitting, row padding, gaps, Flexible Space distribution, and actual clipped-width budgeting.
- Measure active egui style/font and Elements control extents before selecting the final Elements width. Treat approximately `92.0` points as a target, not a promise.
- Preserve the ComboBox truncate boundary and prove group/child/response containment for wide, exact-fit, and narrow cases.

## Out of scope

- New commands, toolbar customization/persistence, command catalogs, icon-library replacement, theme redesign, title-strip work, cursor ownership beyond the toolbar hover response, and editor/core semantics.
- Silently changing the approved default Item order or overflow command routing.
- Claiming an exact 92-point Elements width if active metrics or the clipped row budget require another measurable value.

## Requirements

Applicable wording copied verbatim from [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:90):

- **REQ-WRITING-LOOP-005:** “As an individual screenwriter, I want desktop commands in a consistent toolbar, so common project actions are discoverable without changing the semantic editing model.”
- **AC-WRITING-005.1:** “Given the desktop app is running, the toolbar shall present the typed default Item order New, Open, Save, Save As, Flexible Space, Elements, Flexible Space, and Export PDF.”
- **AC-WRITING-005.3:** “Action Items shall use an icon-first, borderless icon control with a small explicit padding region as the hover/click surface. The centered text label beneath it remains outside that response, retains its normal text color, remains available for accessibility use, and does not add a redundant visible-label tooltip.”
- **AC-WRITING-005.5:** “Given a toolbar width, Space shall consume one deterministic 72-point width, rendered normal Items shall consume their complete group width including label text measured at runtime with the active egui style/font, and Flexible Space Items shall divide the remaining width equally after the shared 4-point horizontal row-padding and 4-point inter-item-gap budget, and a narrow toolbar shall retain the leftmost fitting Items while exposing non-spacing Items through a far-right `>>` text menu. Action entries and Elements role choices in that menu shall remain executable without persisting toolbar layout state; a fully fitting composition shall keep its rightmost Item, including its label, inside the actual clipped toolbar content right edge after panel, frame, and viewport insets are applied.”
- **NFR-WRITING-001:** “Production paths should prefer typed errors and bounded inputs over panic-prone handling; a discovered crash or data-loss defect requires a regression test.”
- **NFR-WRITING-002:** “Core editing, layout, and persistence behavior shall be exercisable without a desktop window for deterministic tests.”

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:58) — the desktop owns a transient typed toolbar; action responses and the Elements group remain presentation-only and route semantic changes through existing adapters.
- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:73) — allocation uses one shared row budget, runtime label measurement, explicit padding/gap/overflow controls, and the actual clipped content edge.
- [`technical-context.md`](../../../blueprints/technical-context.md:31) — presentation state remains above the headless semantic core.

## Current implementation findings

- [`ToolbarItem`](../../../../apps/scriptcraft/src/main.rs:80), [`default_toolbar_items()`](../../../../apps/scriptcraft/src/main.rs:148), and [`toolbar_item_width()`](../../../../apps/scriptcraft/src/main.rs:173) provide the typed composition but no per-item override.
- [`allocate_toolbar()`](../../../../apps/scriptcraft/src/main.rs:347) and [`toolbar()`](../../../../apps/scriptcraft/src/main.rs:735) duplicate the width/gap accounting boundary; [`toolbar_actual_available_width()`](../../../../apps/scriptcraft/src/main.rs:215) already intersects content and clip rectangles and must remain authoritative.
- [`toolbar_action_button()`](../../../../apps/scriptcraft/src/main.rs:604) currently uses `CursorIcon::PointingHand` and owns an icon-plus-padding response; the label is painted separately.
- [`elements_toolbar_item()`](../../../../apps/scriptcraft/src/main.rs:669) constrains the child and ComboBox to the passed width and truncates selected text; the Elements width is currently generic.
- [`toolbar_combo_boundary_ledger()`](../../../../apps/scriptcraft/src/main.rs:246) and the toolbar geometry tests provide an existing deterministic containment harness.
- The source constant [`TOOLBAR_NORMAL_ITEM_WIDTH`](../../../../apps/scriptcraft/src/main.rs:23) is `56.0`, while [`toolbar_manual_geometry_controls_are_narrow_and_explicit()`](../../../../apps/scriptcraft/src/main.rs:1933) expects `46.0`; reconcile this as part of implementation and update only evidence-backed durable wording.

## Implementation plan

1. Refactor toolbar metadata/width resolution so a normal fallback and an Elements-specific override are resolved once per Item and reused by allocation and renderer.
2. Preserve the 72-point Space contract, 4-point row padding/gap budget, Flexible Space equality, leftmost-fit overflow, and clipped-edge reservation while consuming the resolved widths.
3. Tune the interaction geometry so action hover/click is the explicit icon-plus-padding response, Elements has an explicit control/label response boundary, and labels do not gain redundant visible tooltips.
4. Add deterministic tests for measured width selection, Elements truncation/containment, wide/exact/narrow allocation, overflow, and hover ownership. Do not add native-window claims to headless tests.

## Files and systems

- **Create:** Tests only if a separate bounded app test module is needed.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1), this work order, and authorized requirements/desktop-blueprint wording if the resulting contract differs from current text.
- **Avoid changing:** [`crates/scriptcraft-core/src/lib.rs`](../../../../crates/scriptcraft-core/src/lib.rs:1), editing/session policy, persistence, exports, title strip, document cursor behavior, and native project schema.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; `cargo check --workspace`; focused app tests for toolbar allocation/geometry/hover; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`.
- **Deterministic acceptance:** For wide, exact-fit, and narrow actual clipped widths, allocation and rendering use identical resolved widths; the final visible label remains within the intersected clip/content edge; Elements control/group rectangles remain contained; overflow remains leftmost-fit and executable.
- **Manual/external checks:** If a window-capable run is available, inspect action icon hover versus label hover, Elements label/control behavior, and wide/narrow right-edge fit. If unavailable, record that limitation and do not claim native visual success.
- **Acceptance test:** Given the default typed toolbar and an active egui style/font, when the toolbar is laid out at wide, exact-fit, and narrow clipped widths, then action hover is confined to the icon control, Elements uses the measured final override without broad truncation where the budget permits, Flexible Spaces and overflow remain deterministic, and no child or label exceeds the actual clipped content edge.

## Durable-document impact

- **Requirements impact:** `update` — the user-visible hover and per-item allocation behavior is an explicit refinement of AC-WRITING-005.3 and AC-WRITING-005.5.
- **Blueprint impact:** `update` — the transient desktop width-resolution and response-ownership contract needs to name the shared resolved-width boundary and Elements exception.
- **Affected documents/IDs:** [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:90) / REQ-WRITING-LOOP-005 / AC-WRITING-005.3 / AC-WRITING-005.5; [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:58).
- **Rationale and evidence:** Current source already owns the toolbar contract, but its generic width path, source/test mismatch, and action cursor/response geometry do not fully express the requested outcome. Evidence is the current [`main.rs`](../../../../apps/scriptcraft/src/main.rs:23) and focused geometry/ComboBox ledgers.
- **Authorized documentation changes:** Record only final evidence-backed width-resolution, hover ownership, Elements control budget, and clipped-edge behavior. Do not update semantic/core or persistence contracts.

## Dependencies, risks, and handoff

- **Dependencies:** Source/history reconciliation is complete for planning; this WO has no prerequisite implementation WO. WO-002 depends on the final toolbar vertical/available-geometry boundary established here. WO-003 may follow this WO.
- **Risks:** Font metrics and ComboBox intrinsic minimums may make 92 points insufficient or cause narrow overflow; duplicated width accounting may reintroduce edge overruns. Mitigate with measured values, one resolver, and ledgers.
- **Handoff:** Leave the final geometry constants and resolved-width contract discoverable in tests for the title-strip implementer.

## Completion checklist

- [x] Implementation is complete within scope.
- [x] Tests/checks and required gates have evidence.
- [x] Durable-document impact classification is accurate against the diff, and authorized documentation updates are applied.
- [x] Execution log is appended with dispatch and completion session/agent identifiers.
- [x] Fresh validation passed with warnings; work order is `complete`. See [`validation-WO-001.md`](../validations/validation-WO-001.md:1).

## Implementation evidence

- **Dispatch/completion session:** `implementer-wo001-toolbar-geometry-2026-10-27T1552Z`; this was a fresh implementer session for WO-001 only. WO-002, WO-003, and WO-004 were not implemented.
- **Branch evidence:** recorded base `master` resolved to `f5b02d3eb58464f091c57667711f3be738e65e56`; local branch capability succeeded; `dev/2026-10-27-toolbar-and-titlebar-polish` was created from `master` before source changes. Initial and post-creation status both showed only the pre-existing untracked initiative directory.
- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs), [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md), [`toolbar-and-titlebar-polish-v1.md`](../toolbar-and-titlebar-polish-v1.md), this work order, and [`execution-log.md`](../execution-log.md).
- **Implementation:** reconciled the normal Item baseline to 46 points; introduced typed `ToolbarWidthOverride` resolution with a 92-point Elements target and the preserved 72-point Space contract; reused resolved widths through allocation/rendering and existing clipped-width/overflow accounting; kept ComboBox truncation and added containment assertion; retained icon-plus-padding hover ownership, changed toolbar control cursor intent to the standard arrow, and kept labels outside the action response.
- **Focused validation attempt:** `cargo test -p scriptcraft toolbar_` — passed, 26 tests.
- **Required checks:** first `cargo fmt --all -- --check` attempt reported only formatter drift in [`main.rs`](../../../../apps/scriptcraft/src/main.rs); `cargo fmt --all` was run, then `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`, `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` all passed with exit code 0. Full workspace test output reported 65 app unit tests and 44 core tests passing, plus all existing integration suites passing.
- **Comparable diff evidence:** `git status --short --branch` and `git diff --stat` on the initiative branch showed the three source/durable-document files above plus the initiative evidence directory; the diff is limited to toolbar width/hover/containment behavior, focused tests, and the authorized requirements/blueprint wording.
- **Unavailable/manual evidence:** no window-capable native smoke was run; no native hover rendering claim is made. No validation artifact was created. A fresh validator must determine whether this work order can move from `in-review` to `complete`.

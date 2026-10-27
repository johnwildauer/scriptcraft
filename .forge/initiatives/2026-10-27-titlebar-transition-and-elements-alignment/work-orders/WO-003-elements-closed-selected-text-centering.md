# Work Order: WO-003 — Elements Closed Selected-Text Centering

## Metadata

- **Status:** `complete`
- **Assignee:** egui toolbar-control implementer (one fresh session)
- **Phase/order:** 3 of 4; depends on WO-002’s settled toolbar surface geometry
- **Initiative:** [`../titlebar-transition-and-elements-alignment-v1.md`](../titlebar-transition-and-elements-alignment-v1.md:1)
- **Generated filename:** `WO-003-elements-closed-selected-text-centering.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-003.md`](../validations/validation-WO-003.md:1)
- **Fixes:** none
- **Implementer session/agent ID:** `implementer-fresh-wo003-elements-2026-10-27T1738Z`
- **Session exception:** none

## Summary

Center the selected role text in the closed Elements control while leaving opened popup choices left-aligned. Preserve the current observed `128.0`-point width, arrow placement, truncation, keyboard/open behavior, role routing, and accessibility/event ownership; use an installed egui API where possible and isolate only the closed display if a custom presentation is necessary.

## In scope

- Inspect installed egui 0.29 ComboBox/selected-text layout APIs for independent closed-text alignment.
- Implement closed-state selected-text centering at the current `TOOLBAR_ELEMENTS_WIDTH = 128.0` baseline.
- Preserve arrow placement, width budget, truncation behavior, keyboard navigation, popup opening, and transactional `EditingSession::change_role()` routing.
- Keep popup role choices left-aligned and test long labels, all seven roles, exact/wide/narrow allocation, and open/closed layout separately.

## Out of scope

- Changing the 128-point baseline, toolbar allocator policy, titlebar/viewport behavior, popup choice ordering, role semantics, filename plumbing, or broad theme redesign.
- Centering popup choices, changing arrow semantics, replacing the role model, or adding a second semantic text buffer.

## Requirements

Applicable wording copied verbatim from [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:90):

- **AC-WRITING-005.4:** “Given the current caret/focus is within a screenplay element, the Elements Item shall display that element's explicit role using the seven supported labels Scene Heading, Action, Character, Dialogue, Parenthetical, Transition, and Note; choosing a label shall apply the role through the existing transactional session operation and preserve the document on rejection.”
- **AC-WRITING-005.5:** “Given a toolbar width, Space shall consume one deterministic 72-point width, rendered normal Items shall consume their complete group width including label text measured at runtime with the active egui style/font, and Flexible Space Items shall divide the remaining width equally after the shared 4-point horizontal row-padding and 4-point inter-item-gap budget, and a narrow toolbar shall retain the leftmost fitting Items while exposing non-spacing Items through a far-right `>>` text menu. Action entries and Elements role choices in that menu shall remain executable without persisting toolbar layout state; a fully fitting composition shall keep its rightmost Item, including its label, inside the actual clipped toolbar content right edge after panel, frame, and viewport insets are applied. Items with an approved typed width override, including the Elements control, shall use that same resolved width for allocation and rendering without changing the shared padding, gap, flexible-space, overflow, or clipped-edge budget.”

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:59) — Elements derives its selected label from session focus, uses a typed width policy, and routes choices through the core session.
- [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:53) — semantic role ownership and transactional editing remain in the core/session boundary.
- [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:78) — role display is a desktop consumer of explicit semantic elements, not a new role policy.

## Source/API findings

- [`elements_toolbar_item()`](../../../../apps/scriptcraft/src/main.rs:689) currently uses `ComboBox::width(width)`, truncate wrapping, stock `.selected_text(...)`, and left-to-right popup `selectable_value` entries.
- [`ToolbarItem::elements()`](../../../../apps/scriptcraft/src/main.rs:117) resolves [`TOOLBAR_ELEMENTS_WIDTH`](../../../../apps/scriptcraft/src/main.rs:26) to `128.0` for both allocation and rendering; this is the current observed baseline.
- Existing [`toolbar_combo_boundary_ledger()`](../../../../apps/scriptcraft/src/main.rs:264) tests response/child/group containment and should remain authoritative for width safety.
- The parent’s earlier approximate 92-point target is historical and must not override the current 128-point local change.

## Implementation plan

1. Inspect installed egui ComboBox/WidgetText APIs and choose a closed-only alignment mechanism if available.
2. If stock ComboBox cannot independently center the closed selected text, isolate a custom closed display while retaining the stock popup or equivalent role-choice path; do not move popup text alignment.
3. Measure selected galley, arrow, control, group, and clip rectangles at 128 points; preserve truncation and prevent text/arrow collision.
4. Add deterministic tests for every supported role, long/truncated labels, closed centering, popup left alignment, width containment, opening/keyboard choice routing, and rejected role-change preservation.

## Files and systems

- **Create:** no new runtime file expected; focused tests may be added in app source or an app test module.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:689) and this WO; blueprint only if the durable UI contract changes.
- **Avoid changing:** `TOOLBAR_ELEMENTS_WIDTH`, core role transitions, titlebar/viewport code, filename/title state, and popup alignment policy.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; focused Elements/ComboBox tests; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`.
- **Deterministic acceptance:** at 128 points, the closed selected galley’s center matches the control’s intended text region; arrow remains correctly placed; long labels truncate without overflow; opened choices are left-aligned; all seven roles route through the existing session operation; width ledgers remain within allocation and clip bounds.
- **Manual/native-smoke:** In a window-capable macOS run, inspect closed alignment at each role and a long label, open the popup and confirm left-aligned choices, then select a role with mouse and keyboard. Record unavailable native evidence as a warning, not a pass claim.
- **Acceptance test:** Given a focused role, when the Elements control is closed, then its selected text is centered within the 128-point closed display without arrow collision; when opened, choices remain left-aligned and selection preserves existing transactional behavior.

## Durable-document impact

- **Requirements impact:** `verify unchanged` — [`AC-WRITING-005.4`](../../../requirements/features/screenplay-writing-loop.md:109) defines semantic role display/routing and [`AC-WRITING-005.5`](../../../requirements/features/screenplay-writing-loop.md:115) defines the typed width budget; neither requires popup centering.
- **Blueprint impact:** `update` — [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:72) should record the newly verified distinction that only the closed selected display is centered while popup choices remain left-aligned, without changing semantic ownership.
- **Affected documents/IDs:** desktop Elements Item contract; REQ-WRITING-LOOP-005 / AC-WRITING-005.4–005.5 verification.
- **Rationale and evidence:** Current source uses stock selected text and popup choices in one ComboBox; the requested behavior is a new presentation detail, while the current 128-point width is explicitly observed in source and the closed parent’s 92-point target is obsolete for this follow-up.
- **Authorized documentation changes:** Add only closed-state selected-text centering, preserved 128-point baseline, arrow/truncation constraints, and left-aligned popup choices; do not alter role semantics or claim a width change.

## Dependencies, risks, and handoff

- **Dependencies:** WO-002 establishes final shared surface/toolbar geometry; WO-004 consumes the closed/open evidence.
- **Risks:** egui may not expose independent closed alignment; custom display can steal popup/keyboard ownership or collide with the arrow; 128 points may still truncate long labels.
- **Handoff:** provide closed/open rectangle measurements and the chosen stock/custom technique to WO-004.

## Completion checklist

- [x] Implementation is complete within scope.
- [x] Tests/checks and bounded native evidence have been recorded.
- [x] Durable-document impact classification is accurate against the diff.
- [x] Execution log is appended with fresh dispatch/completion identifiers.
- [x] Work order is `in-review`; `complete` requires fresh validation.

## Implementation evidence

- **Dispatch/completion session:** `implementer-fresh-wo003-elements-2026-10-27T1738Z`; fresh implementer session for WO-003 only. WO-004 was not implemented.
- **Branch/status evidence:** `git status --short --branch` confirmed `dev/2026-10-27-titlebar-transition-and-elements-alignment`. Existing modified [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:1), and untracked initiative evidence were preserved; no reset, checkout, or destructive Git action was used.
- **API finding and implementation:** Installed egui 0.29.1 [`ComboBox`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/containers/combo_box.rs:40) exposes width, selected text, wrap mode, icon, and stock popup APIs but no independent closed selected-text alignment. The implementation keeps the stock ComboBox button, arrow, response/ID, popup, selectable-value routing, and keyboard/open path, passes an empty stock selected galley, and paints a separate centered closed galley only when `show_ui` reports the popup closed. The galley uses `TextWrapMode::Truncate` and an arrow-excluded text rectangle.
- **Geometry/budget evidence:** [`TOOLBAR_ELEMENTS_WIDTH`](../../../../apps/scriptcraft/src/main.rs:26) remains `128.0`; [`elements_closed_text_rect()`](../../../../apps/scriptcraft/src/main.rs:307) reserves button padding, icon spacing, and icon width, while the stock ComboBox retains arrow placement and the existing width allocation/debug bound. Long-label geometry is constrained to the available closed text region, and popup choices continue through the existing left-aligned `selectable_value` list.
- **Deterministic tests:** Added closed centering/arrow separation, long-label truncation/containment, and closed-versus-stock-popup policy coverage; existing all-seven-role, width-boundary, role-routing, rejection-preservation, keyboard/open, and overflow Elements tests remain passing.
- **Durable documents:** Requirements are `verify unchanged` against AC-WRITING-005.4/005.5. Blueprint impact is `update`: [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:73) records closed-only selected-text centering, preserved 128-point/arrow/truncation behavior, and left-aligned popup choices without changing semantic ownership.
- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:1), this work order, and [`execution-log.md`](../execution-log.md:1). No requirements file, core crate, WO-004 artifact, or validation artifact was changed.
- **Validation attempts/results:** `cargo fmt --all -- --check`, focused `cargo test -p scriptcraft elements -- --nocapture` (11 passed), `cargo check --workspace`, `cargo test --workspace` (75 app tests, 45 core tests, and integration suites passed), `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` all passed.
- **Native smoke limitation:** No window-capable macOS run was available. Closed/open visual inspection at each role, native cursor observation, and native keyboard/mouse smoke are therefore unavailable; this is a warning under project policy, not a pass claim or implementation blocker.
- **Comparable diff:** `git status --short --branch`, `git diff --stat`, source/blueprint diff inspection, and `git diff --check` provide working-tree comparison evidence. The source diff includes the already validated WO-001/WO-002 baseline plus this closed Elements presentation/test change; no WO-004 behavior was introduced.
- **Next state:** `complete`; fresh validation passed with warnings. See [`validation-WO-003.md`](../validations/validation-WO-003.md:1).

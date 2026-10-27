# Work Order: V2-WO-002 — Elements Cursor Ownership

## Metadata

- **Status:** `complete`
- **Assignee:** egui Elements ownership implementer (fresh session)
- **Phase/order:** 2 of 4; depends on V2-WO-001 only for shared diagnostics, and may execute after its contract is recorded
- **Initiative:** [`../titlebar-transition-and-elements-alignment-v2.md`](../titlebar-transition-and-elements-alignment-v2.md:1)
- **Generated filename:** `V2-WO-002-elements-cursor-ownership.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-V2-WO-002.md`](../validations/validation-V2-WO-002.md:1)
- **Fixes:** none
- **Implementer session/agent ID:** `implementer-fresh-elements-cursor-2026-10-27T1912Z`
- **Session exception:** none

## Summary / one outcome

Identify the smallest competing response/label/ComboBox ownership path that produces the Elements text I-beam and correct only that path. Preserve the stock arrow, closed centered selected galley, left-aligned popup, keyboard/open behavior, role routing, document I-beam, and the fixed `TOOLBAR_ELEMENTS_WIDTH = 128.0` baseline.

## In scope

- Build a response/cursor ledger for outer Elements group, stock ComboBox response/ID/arrow, selected galley, decorative label, popup rows, group gap, and document response.
- Use installed egui 0.29 behavior and native pointer observations to distinguish label, stock ComboBox, parent ordering, and document overlap.
- Make the smallest local ownership correction, preferably a non-sensing decorative label plus authoritative standard-arrow ownership; retain stock ComboBox unless evidence requires the smallest supported wrapper.
- Test closed/open geometry, all seven roles, long-label truncation, popup alignment, pointer regions, keyboard/open behavior, role routing, and rejection preservation.

## Out of scope

- Replacing the ComboBox wholesale, adding a response to the painted galley, changing role semantics, popup ordering/alignment, toolbar allocation, titlebar/viewport behavior, full-screen, filename plumbing, or width.

## Requirements

- **AC-WRITING-005.4:** “Given the current caret/focus is within a screenplay element, the Elements Item shall display that element's explicit role using the seven supported labels Scene Heading, Action, Character, Dialogue, Parenthetical, Transition, and Note; choosing a label shall apply the role through the existing transactional session operation and preserve the document on rejection.”
- **AC-WRITING-005.5:** “Given a toolbar width, Space shall consume one deterministic 72-point width, rendered normal Items shall consume their complete group width including label text measured at runtime with the active egui style/font, and Flexible Space Items shall divide the remaining width equally after the shared 4-point horizontal row-padding and 4-point inter-item-gap budget, and a narrow toolbar shall retain the leftmost fitting Items while exposing non-spacing Items through a far-right `>>` text menu. Action entries and Elements role choices in that menu shall remain executable without persisting toolbar layout state; a fully fitting composition shall keep its rightmost Item, including its label, inside the actual clipped toolbar content right edge after panel, frame, and viewport insets are applied. Items with an approved typed width override, including the Elements control, shall use that same resolved width for allocation and rendering without changing the shared padding, gap, flexible-space, overflow, or clipped-edge budget.”
- **AC-WRITING-005.7:** “Toolbar action controls and the Elements control/label surface shall request the standard arrow cursor, while the existing whole-document editing response shall request the text/I-beam cursor. Decorative labels and painted page rectangles shall not create competing cursor owners.”

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:59) — Elements uses typed width, stock popup/role routing, and decorative label ownership.
- [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:53) — role policy and transactional mutation remain in the core/session.
- [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:78) — desktop display consumes explicit roles.

## Source/API findings

- [`elements_toolbar_item()`](../../../../apps/scriptcraft/src/main.rs:714) retains stock ComboBox response/ID/popup behavior and paints the closed centered galley separately.
- [`toolbar_cursor_icon()`](../../../../apps/scriptcraft/src/main.rs:1706) requests the standard arrow/default cursor; [`live_document_surface()`](../../../../apps/scriptcraft/src/main.rs:1126) requests text for the whole document response.
- The decorative `ui.label(item.label)` remains a child response path; a parent cursor request is not assumed to override a hovered child.
- Installed egui 0.29 ComboBox has no independent closed selected-text alignment API; do not replace the stock owner merely to gain cursor control.

## Implementation guidance

1. Instrument/record response rectangles and cursor intents before selecting a fix.
2. Remove the smallest decorative competing response or use a non-sensing paint/allocation path, while preserving the ComboBox response and ID.
3. If stock internals still win incorrectly, apply the narrowest supported local ordering/wrapper; do not add a second semantic control.
4. Verify the document response does not overlap the toolbar in actual geometry.

## Files and systems

- **Create:** focused cursor/response ledger tests only if needed.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:714) and this work order; blueprint only if the durable cursor-owner contract changes.
- **Avoid changing:** `TOOLBAR_ELEMENTS_WIDTH`, role/session code, popup alignment, viewport/transition behavior, and v1 artifacts.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; `cargo test -p scriptcraft elements -- --nocapture`; focused cursor/toolbar tests; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`.
- **Deterministic tests:** standard arrow intent for arrow, selected-text, label, group-gap, and action regions; text intent only for document response; closed selected galley remains centered in arrow-excluded rect; popup policy remains left-aligned; width is exactly 128 points; all seven roles, keyboard/open, routing, and rejection preservation remain green.
- **Native-smoke protocol:** on usable macOS, move pointer separately over arrow, closed text, decorative label, group gap, popup rows, and document page; record cursor shape and response/clip rectangles; open with mouse and keyboard, select a role, and repeat for a long label. Missing display/pointer evidence is recorded as unavailable, never inferred from source tests.
- **Acceptance test:** Given the pointer is over any closed Elements surface region, when no popup/document overlap exists, then the standard arrow is shown; when the popup opens, choices remain left-aligned and stock keyboard/role routing remains intact; the document alone shows I-beam.

## Dependencies, risks, and handoff

- **Dependencies:** V2-WO-001’s shared geometry/diagnostic convention; V2-WO-004 consumes this ownership ledger. V2-WO-003 may run after this WO because it concerns a separate surface ownership path.
- **Risks:** egui child response precedence may differ from source intuition; an overlay may steal ComboBox activation; native cursor evidence may be unavailable.
- **Handoff:** provide response ownership table and closed/open behavior evidence to V2-WO-004.

## Durable-document impact

- **Requirements impact:** `verify unchanged` — AC-WRITING-005.4, 005.5, and 005.7 already state role routing, width budget, and cursor ownership.
- **Blueprint impact:** `verify unchanged` by default; `update` only if the confirmed owner differs from [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:121).
- **Affected documents/IDs:** REQ-WRITING-LOOP-005 / AC-WRITING-005.4–005.7; desktop Elements/cursor contract.
- **Rationale and evidence:** current source identifies a decorative label as the strongest competing response, but native ownership remains to be measured.
- **Authorized documentation changes:** only the smallest evidence-backed ownership wording; no semantic or width change.

## Completion checklist

- [x] Implementation and ownership evidence are complete within scope.
- [x] Deterministic checks have results; bounded native smoke is recorded unavailable.
- [x] Fresh validation passed with a non-blocking native-smoke warning; work order is `complete`. See [`validation-V2-WO-002.md`](../validations/validation-V2-WO-002.md:1).

## Implementation evidence — 2026-10-27

- **Source correction:** [`elements_toolbar_item()`](../../../../apps/scriptcraft/src/main.rs:843) still uses the stock [`ComboBox`](../../../../apps/scriptcraft/src/main.rs:854) response/ID, arrow, popup, keyboard path, and role rows. The decorative `Elements` caption is now allocated and painted without a widget response; no response was added to the closed selected galley. The existing group and ComboBox responses retain the standard arrow cursor.
- **Response/cursor ledger:** deterministic helper [`elements_cursor_ledger()`](../../../../apps/scriptcraft/src/main.rs:447) records outer group, stock ComboBox, closed centered galley, decorative caption, popup row, group gap, and document regions. Closed/control regions resolve to `CursorIcon::Default`; the document region resolves to `CursorIcon::Text`. [`toolbar_combo_boundary_ledger()`](../../../../apps/scriptcraft/src/main.rs:396) verifies the decorative allocation remains inside the group without creating a response.
- **Preserved contracts:** `TOOLBAR_ELEMENTS_WIDTH` remains `128.0`; closed text remains centered in the arrow-excluded rectangle; popup choices remain stock/left-aligned; all seven roles, keyboard/open route, transactional role mutation, and rejection preservation tests remain green.
- **Automated evidence:** `cargo fmt --all -- --check`; focused Elements tests (13 passed); `cargo check --workspace`; `cargo test --workspace` (79 app tests, 45 core tests, integration suites, and doc-tests); `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; and `git diff --check` all exited 0.
- **Native smoke:** unavailable. This macOS session has no usable display/pointer automation evidence; no native cursor shape, popup, keyboard, or overlap claim is made. Per policy this is recorded as unavailable, not inferred as pass.
- **Changed-file boundary:** implementation changes are limited to [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:843), plus this work-order and the execution log. Requirements and blueprints are `verify unchanged`. Existing unrelated V2-WO-001 working-tree changes remain present and are not attributed to this WO.
- **Diff/next state:** `git diff --check` passed. The next state is fresh validation of this WO; the pre-existing placeholder [`validation-V2-WO-002.md`](../validations/validation-V2-WO-002.md:1) was not edited or used as validation evidence.

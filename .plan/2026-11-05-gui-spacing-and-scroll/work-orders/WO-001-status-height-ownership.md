# WO-001 — Explicit status-row and panel-height ownership

- **Status:** Implementation submitted for user review; not user-validated or accepted
- **Initiative:** [`gui-spacing-and-scroll.md`](../gui-spacing-and-scroll.md)
- **Depends on:** None
- **Validation owner:** User; no agent-run builds, checks, tests, lint, formatting checks, manual verification, reviews, or independent validators

## Outcome

Make ordinary status-bar height locally understandable and adjustable through one status-row setting and the existing vertical insets, while preserving the current effective appearance and optional Diagnostics behavior.

The normal panel currently resolves to 28 logical points: an 18-point framework interaction row plus 5 points above and below. There is no status-local row-height setting and no app-provided panel default height.

## Current source boundary

- [`PANEL_VERTICAL_INSET`](../../../apps/scriptcraft/src/status_bar.rs:26) already owns the symmetric panel inset.
- [`show_impl_with_appearance()`](../../../apps/scriptcraft/src/status_bar.rs:1259) obtains ordinary row height from [`ui.spacing().interact_size.y`](../../../apps/scriptcraft/src/status_bar.rs:1282).
- [`Panel::bottom()`](../../../apps/scriptcraft/src/main.rs:2564) supplies no explicit default or exact height, so egui derives its ordinary horizontal-panel size from interaction metrics and frame margins.
- [`ENABLE_DIAGNOSTICS`](../../../apps/scriptcraft/src/status_bar.rs:56) is production-off. Its explicitly enabled development presentation may add the existing disclosure content and must not be forced into the ordinary one-row allocation.

## Scope

1. Add [`STATUS_ROW_HEIGHT`](../../../apps/scriptcraft/src/status_bar.rs:20) to status presentation, initialized to 18 logical points as an agent-default extraction of the current effective value.
2. Add a status-owned default-panel-height helper derived from the row setting plus both existing vertical insets. Do not add an independent total-height constant that could disagree with its components.
3. Use the status row setting for the ordinary row rectangle and bounded control regions. Audit status-internal uses of framework interaction height so icons, slider, percentage, page counter, center message, hover painting, and accessibility bounds remain coherent within the row.
4. Supply the derived default height to the bottom panel in [`show_application_panels()`](../../../apps/scriptcraft/src/main.rs:2500). Use default sizing rather than exact sizing so the explicitly enabled Diagnostics presentation retains its existing expansion path.
5. Preserve the current normal initial height, status-local foreground overrides, semantic palette, right-group priorities, mirrored center region, visible-page behavior, zoom controls, accessible names, and command routing.
6. Keep the change desktop-local. Do not mutate global egui spacing or toolbar sizing.

## Non-goals

- No visual redesign or intentional change to the current normal height.
- No changes to status text, icons, colors, zoom limits, page tracking, or Diagnostics production default.
- No promise that arbitrarily small manual row values preserve usable hit targets or unclipped content.
- No screenplay-core, FDX, persistence, pagination, editor, toolbar, or export changes.
- No new automated desktop-test target and no agent execution of validation.

## Structural acceptance for user review

1. One status-local row setting and the existing vertical insets are the only ordinary panel-height inputs.
2. The initial effective normal height remains unchanged.
3. The panel uses default rather than exact sizing, allowing the explicitly enabled Diagnostics disclosure to retain its existing behavior.
4. Status controls remain bounded, nonoverlapping, and accessible at supported widths.
5. No global or toolbar framework metrics are changed.

## Durable records

- Update [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md:668) so the status contract describes named row/inset ownership, derived ordinary panel sizing, height stability, and the separate optional Diagnostics expansion behavior.
- Update [`technical-context.md`](../../../.forge/blueprints/technical-context.md:140) so the status architecture identifies the same ownership and default-versus-exact sizing boundary.
- Keep these durable records limited to the lasting product and technical contract. They must not mention this work order, implementation chronology, or numeric developer-tuning values.
- Record implementation details and evidence limitations only in this draft and the initiative overview; do not turn requirements or blueprints into a development changelog.

## User-owned review requests

- Confirm the ordinary status bar retains its current height and remains stable through scrolling, page zoom, long operation/error messages, and wide/compact window resizing.
- Confirm Timer, Focus Mode, Appearance, slider, percentage, page counter, and center message remain visible, nonoverlapping, and interactive.
- If desired, adjust the new row setting locally and verify that ordinary panel height follows the row plus inset derivation without another hidden total-height source.
- If Diagnostics is explicitly enabled for development, confirm its disclosure still allocates and operates as before; confirm production-off mode has no Diagnostics row or accessibility target.
- Run any desired workspace compilation and behavior checks and record outcomes separately; no agent result is claimed.

## Implementation-submission handoff

- **Branch:** `dev/2026-11-05-gui-spacing-and-scroll`, created from the inspected current checkout without staging, committing, stashing, or discarding its existing untracked initiative files.
- **Source changes:** [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs) adds `presentation::STATUS_ROW_HEIGHT = 18.0`, retains the existing `PANEL_VERTICAL_INSET = 5`, derives the ordinary default panel height as row height plus both insets (28 logical points initially), and uses the row setting for the ordinary row rectangle, slider height, icon hit/control geometry, and reusable text/icon item interaction height. No independent total-height setting or global spacing mutation was added.
- **Caller change:** [`main.rs`](../../../apps/scriptcraft/src/main.rs) supplies `status_bar::default_panel_height()` to the status bottom panel through `.default_size(...)`, preserving default rather than exact sizing so enabled Diagnostics can expand through normal panel allocation.
- **Durable contracts:** [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md) now states row/inset ownership, derived default sizing, ordinary-height stability, and optional Diagnostics expansion. [`technical-context.md`](../../../.forge/blueprints/technical-context.md) describes the same enduring ownership and behavior without work-order chronology or numeric tuning values.
- **Evidence limitations:** Source and planning/documentation edits only. No build, check, test, lint, formatting command, manual verification, independent review, or validator was run. Compilation, framework compatibility, rendered appearance, accessibility bounds, interactions, and Diagnostics expansion remain unverified and user-owned.
- **Review state:** Submitted for user review only; no user validation or acceptance is claimed.

# WO-008 — Remove the toolbar's trailing blank allocation

- **Status:** Implementation submitted for user validation; no validation or acceptance recorded
- **Initiative:** [`boundary-and-border-investigation.md`](../boundary-and-border-investigation.md)
- **Depends on:** Existing toolbar and asymmetric Script View boundary implementations
- **Validation owner:** User; no agent-run builds, checks, tests, lint, formatting checks, manual verification, reviews, or independent validators

## Outcome

Remove the remaining blank toolbar allocation between the Item group and the toolbar's lower divider so Script View begins immediately below the toolbar panel.

Make the correction at the actual owning constant rather than compensating again in Script View.

## Current finding

The submitted source sets [`TOOLBAR_BOTTOM_PADDING`](../../../apps/scriptcraft/src/toolbar.rs:18) to zero (previously 6 logical points). It remains added after the Item group at [`ui.add_space()`](../../../apps/scriptcraft/src/toolbar.rs:923) and included in [`toolbar_content_height()`](../../../apps/scriptcraft/src/toolbar.rs:1011), so the existing exact-height derivation now consists of top padding plus the Item-group height.

The toolbar framework separator is already disabled, the central Script Workspace frame has no margin, and Script View top content margin is zero. This bottom-padding allocation is the remaining source-owned blank region.

## Scope

1. Set [`TOOLBAR_BOTTOM_PADDING`](../../../apps/scriptcraft/src/toolbar.rs:18) to zero.
2. Retain the existing call site and height derivation so the exact panel height automatically becomes top padding plus Item group height with no trailing blank component.
3. Preserve current toolbar top padding, Item group metrics, caption positions, Elements optical offset, horizontal allocation, icon hover geometry, divider width/color, commands, overflow, and accessibility behavior.
4. Preserve Script View's zero top margin, margin-free frame, disabled fades, bottom page-end padding, page publication, and zoom geometry. Do not add a compensating negative margin or alter page positions directly.
5. Confirm in source that the toolbar divider remains at the resized panel's lower edge and the central panel still follows the top panel without another application spacer.
6. Reconcile the durable toolbar boundary contract and record the implementation handoff in this draft and the investigation note.

## Non-goals

- No change to toolbar divider thickness or color in this order.
- No change to toolbar top padding, captions, icons, or user-adjusted Item settings.
- No Script View, status, page-border, screenplay-core, persistence, pagination, or export changes.
- No guarantee that hover expansion cannot be clipped by the smaller panel; preserve the existing hover geometry rather than hiding a regression through unrelated changes.
- No agent-run validation.

## Structural acceptance for user review

1. The toolbar's trailing padding setting contributes zero allocation.
2. Exact panel height contains only the remaining explicit top and Item-group components.
3. Script View begins immediately after the resized toolbar and retains zero top spacing.
4. Current toolbar divider, captions, interaction geometry, and all unrelated surfaces remain unchanged.

## Durable records

- Update the toolbar geometry requirements in [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md:534) to state that the Item group is not followed by an additional blank toolbar allocation before Script View.
- Update [`technical-context.md`](../../../.forge/blueprints/technical-context.md:28) to identify the exact panel-height components and consecutive toolbar/Script View boundary ownership.
- Keep durable records free of work-order identifiers, implementation chronology, and numeric spacing values. Exact values and handoff evidence belong only in this planning directory.

## User-owned review requests

- Confirm there is no blank band between the toolbar Item group/lower divider and the first page.
- Confirm captions, Elements alignment, and the toolbar divider remain visible and correctly positioned.
- Check icon hover rectangles at the lower edge, wide/compact layouts, overflow, and role popups.
- Confirm Script View scrolling, bottom page-end allowance, zoom anchoring, and page status remain unchanged.
- Run any desired workspace checks and record outcomes separately; no agent result is claimed.

## Implementation-submission handoff

- **Branch:** `dev/2026-11-05-gui-spacing-and-scroll`; the existing checkout and prior work-order handoffs and user-owned source adjustments were preserved. No branch or history operations were performed.
- **Exact source change:** [`TOOLBAR_BOTTOM_PADDING`](../../../apps/scriptcraft/src/toolbar.rs:18) changed from `6.0` to `0.0`. The existing [`ui.add_space()`](../../../apps/scriptcraft/src/toolbar.rs:923) call and [`toolbar_content_height()`](../../../apps/scriptcraft/src/toolbar.rs:1011) derivation were retained; top padding and Item-group geometry were not changed by this submission.
- **Resulting allocation:** The exact toolbar panel height is top padding plus Item-group height. Under the noted current interaction metrics, the group is 36 logical points and the panel is 40 logical points; the toolbar divider remains painted at the panel's lower edge. The consecutive central Script View panel retains its margin-free frame and zero top content margin, with no application spacer between the panels.
- **Files changed for this submission:** [`toolbar.rs`](../../../apps/scriptcraft/src/toolbar.rs:18), [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md:612), [`technical-context.md`](../../../.forge/blueprints/technical-context.md:51), this draft, and [`boundary-and-border-investigation.md`](../boundary-and-border-investigation.md:1). No WO-009 border change was made.
- **Evidence limitations:** No build, check, test, lint, formatting command, manual verification, reviewer, validator, or user validation was run or claimed. This is an implementation submission only; user validation and acceptance remain outstanding.

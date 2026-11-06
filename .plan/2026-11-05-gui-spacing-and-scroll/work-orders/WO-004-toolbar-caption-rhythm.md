# WO-004 — Shared toolbar caption rhythm and complete panel height

- **Status:** Implementation submitted for user review; not user-validated or accepted
- **Initiative:** [`gui-spacing-and-scroll.md`](../gui-spacing-and-scroll.md)
- **Depends on:** [`WO-003`](WO-003-script-view-scroll-edges.md) under sequential execution; the toolbar change is otherwise independent
- **Validation owner:** User; no agent-run builds, checks, tests, lint, formatting checks, manual verification, reviews, or independent validators

## Outcome

Remove Elements caption clipping, establish one explicit control-to-caption spacing setting shared with action Items, and make the exact toolbar panel height account for its complete vertical composition.

The correction must not alter item widths, horizontal allocation, icon graphics, role selection, overflow, commands, or accessibility behavior.

## Current source findings

- [`toolbar_content_height()`](../../../apps/scriptcraft/src/toolbar.rs:964) returns two framework interaction heights plus [`TOOLBAR_BOTTOM_PADDING`](../../../apps/scriptcraft/src/toolbar.rs:17), currently producing a 42-point exact panel allocation.
- [`toolbar()`](../../../apps/scriptcraft/src/toolbar.rs:812) also applies [`TOOLBAR_TOP_PADDING`](../../../apps/scriptcraft/src/toolbar.rs:14). Its actual vertical composition is therefore 6 points above, a 36-point Item group, and 6 points below: 48 points total.
- [`elements_toolbar_item()`](../../../apps/scriptcraft/src/toolbar.rs:619) allocates the 36-point Item group, then combines the ordinary 18-point closed ComboBox row, the framework's current 3-point vertical item gap, and an 18-point caption allocation at [`ui.allocate_space()`](../../../apps/scriptcraft/src/toolbar.rs:701). The nominal 39-point stack exceeds its group before the outer panel shortfall is considered.
- [`toolbar_action_button()`](../../../apps/scriptcraft/src/toolbar.rs:537) separately uses the framework item gap to position action captions. The spacing is therefore not Elements-specific and is not owned by role-option row padding.

## Scope

1. Add [`TOOLBAR_CONTROL_CAPTION_GAP`](../../../apps/scriptcraft/src/toolbar.rs:7), initialized to 1 logical point as an agent-default reduction from the current effective 3-point gap.
2. Apply that shared setting to both action graphic/caption composition and Elements closed-control/caption composition.
3. Allocate explicit control, gap, and remaining-caption rectangles within the existing two-interaction-height Item group. Remove caption dependence on implicit vertical item spacing and avoid adding a second full interaction row beyond the group.
4. Correct [`toolbar_content_height()`](../../../apps/scriptcraft/src/toolbar.rs:964) to include top padding, Item group height, and bottom padding. Preserve the existing 6/36/6-point components, intentionally correcting the exact panel allocation from 42 to 48 points.
5. Reuse one Item-group height calculation at panel, action, and Elements allocation/render call sites so their vertical budgets cannot diverge silently.
6. Keep icon glyph placement and per-edge hover/click padding independent from the new visual caption gap. Keep ComboBox button padding and Elements role-option row padding independent as well.
7. Preserve item widths, flexible spaces, narrow-width overflow, right-edge fitting, enabled/disabled presentation, Material icons, closed selected-role text, role-popup sizing and selection, keyboard behavior, cursors, commands, accessible names, and theme palettes.
8. Update the durable toolbar geometry contract and record the implementation handoff in the initiative records.

## Non-goals

- No toolbar Item additions, removals, reordering, enablement changes, or horizontal spacing changes.
- No ComboBox replacement, popup redesign, role catalog change, or role-option typography/padding change.
- No icon size, glyph baseline, hover rectangle, or status-bar geometry changes.
- No screenplay-core, editor, FDX, pagination, persistence, or export changes.
- No new automated desktop-test target and no agent execution of validation.

## Structural acceptance for user review

1. One named setting owns the visual gap between an Item's control/graphic and its caption for both action Items and Elements.
2. Each Item's control, gap, and caption fit within the shared Item-group allocation without relying on implicit framework item spacing.
3. Exact toolbar panel height includes all top, group, and bottom components and agrees with actual allocation.
4. The initial caption gap and corrected outer panel height match the reviewed implementation values, with no horizontal allocation changes.
5. Role selection, popup behavior, overflow, commands, accessibility, and presentation precedence remain unchanged.

## Durable records

- Update the toolbar requirements in [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md:534) to state that action and Elements captions use explicit shared vertical rhythm, fit within a common Item-group budget, and remain fully contained by the toolbar panel.
- Update the toolbar architecture in [`technical-context.md`](../../../.forge/blueprints/technical-context.md:28) to identify the shared caption-gap owner and the complete top/group/bottom panel-height derivation, while preserving separate icon-hover, ComboBox, and role-option padding ownership.
- Keep these durable records limited to lasting behavior and ownership. They must not mention this work order, implementation chronology, or numeric spacing, padding, font, or panel-height values.
- Record exact implementation values and evidence limitations only in this draft and the initiative overview, not in requirements or blueprints.

## User-owned review requests

- Move the caret through representative screenplay roles and confirm every closed Elements label and the Elements caption remain fully visible with the intended small gap.
- Confirm New, Open, Save, Save As, Show/Hide, Export Text, and Export PDF captions are fully visible and consistently spaced below their graphics.
- Inspect wide, compact, and overflow layouts. Confirm flexible spaces, right-edge fitting, item widths, and overflow entries remain unchanged.
- Exercise Elements pointer/keyboard selection and popup dismissal; confirm popup height, role rows, Unfold More icon, and transactional role changes remain intact.
- Recheck icon hover/click rectangles, disabled-item inertness, cursors, accessible names, and enabled commands.
- Confirm toolbar height remains stable and no caption pixels are clipped at the panel's lower edge.
- Run any desired workspace compilation and behavior checks and record outcomes separately; no agent result is claimed.

## Implementation-submission handoff

- **Branch:** `dev/2026-11-05-gui-spacing-and-scroll`, the existing initiative branch. WO-001 through WO-003, their planning handoffs, and the pre-existing checkout work were preserved. Nothing was staged, committed, stashed, discarded, merged, or rebased.
- **Exact source changes:** [`toolbar.rs`](../../../apps/scriptcraft/src/toolbar.rs) adds `TOOLBAR_CONTROL_CAPTION_GAP = 1.0` logical point and uses it for both action graphic/caption composition and the Elements control/caption composition. The shared Item-group helper derives `2 × interact_size.y`, currently `2 × 18 = 36` logical points, and is reused by the panel row, action Items, and Elements. Action captions are centered in the remainder below the icon-control rectangle; Elements explicitly allocates its ComboBox row, the shared gap, and the remaining caption rectangle. Child vertical item spacing is disabled only for these caption allocations, after the ComboBox and its popup have completed their normal layout. Current allocation is 18 points for the control, 1 point for the gap, and 17 points for the caption. The exact panel-height helper now derives top padding + shared Item-group height + bottom padding: `6 + 36 + 6 = 48` logical points, correcting the previous 42-point exact allocation. Item widths, horizontal allocation, icon graphics/hover padding, ComboBox button padding, role-option padding, and popup behavior were not intentionally changed.
- **Durable records:** [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md) and [`technical-context.md`](../../../.forge/blueprints/technical-context.md) now state the lasting shared caption rhythm, common Item-group budget, complete panel containment, and separate ownership of icon-hover, ComboBox, and role-option padding. The durable toolbar wording contains no work-order identifiers, implementation chronology, or numeric spacing, padding, font, or panel-height tuning values.
- **Files changed for this submission:** [`apps/scriptcraft/src/toolbar.rs`](../../../apps/scriptcraft/src/toolbar.rs), [`.forge/requirements/features/screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md), [`.forge/blueprints/technical-context.md`](../../../.forge/blueprints/technical-context.md), this WO-004 draft, and the initiative overview. Earlier handoffs remain intact.
- **Evidence limitations:** No build, check, test, lint, formatting command, manual verification, independent review, or validator was run. Compilation, framework/API compatibility, visual geometry, toolbar height, action caption visibility, Elements selection/popup behavior, overflow layouts, commands, and accessibility remain unverified and user-owned. This is an implementation submission only; no user validation or acceptance is claimed.

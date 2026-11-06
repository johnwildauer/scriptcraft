# WO-005 — Center and vertically align toolbar captions

- **Status:** Implementation submitted for user validation; no validation or acceptance recorded
- **Initiative:** [`follow-up-refinements.md`](../follow-up-refinements.md)
- **Depends on:** Existing WO-004 implementation and subsequent user-owned source adjustments
- **Validation owner:** User; no agent-run builds, checks, tests, lint, formatting checks, manual verification, reviews, or independent validators

## Outcome

Make the Elements caption horizontally centered and nominally coincident with ordinary action captions, while exposing an Elements-only vertical adjustment for optical fine-tuning.

At the default adjustment, both caption types should use the same group-relative rectangle, horizontal center, vertical center, font, and shared control-to-caption gap.

## Current findings

- Ordinary action captions occupy the remaining group rectangle below the nominal icon-control slot and shared gap, then paint through a centered galley at [`caption_rect.center()`](../../../apps/scriptcraft/src/toolbar.rs:596).
- Elements calculates its caption allocation from [`combo_response.rect.height()`](../../../apps/scriptcraft/src/toolbar.rs:713), so the stock closed ComboBox response can shift the caption center.
- Elements paints at [`label_rect.left_center()`](../../../apps/scriptcraft/src/toolbar.rs:724) with [`Align2::LEFT_CENTER`](../../../apps/scriptcraft/src/toolbar.rs:725), explicitly producing left alignment.
- The ordinary [`TOOLBAR_CONTROL_CAPTION_GAP`](../../../apps/scriptcraft/src/toolbar.rs:15) is reported satisfactory and must retain its current value.

## Scope

1. Add [`TOOLBAR_ELEMENTS_CAPTION_VERTICAL_OFFSET`](../../../apps/scriptcraft/src/toolbar.rs:15), initialized to zero. It applies only to the decorative Elements caption, not to the ComboBox, popup, action Items, or shared caption gap.
2. Introduce one toolbar caption-rectangle helper derived from the Item group rectangle, nominal control-slot height, and existing shared gap. Its result should match the ordinary action caption rectangle.
3. Use that helper for ordinary action captions without intentionally changing their current geometry.
4. Use the same nominal control-slot height and helper for Elements. Do not use the actual ComboBox response height to determine nominal caption placement.
5. Apply the Elements-only vertical offset to its caption rectangle. Keep horizontal positioning independent of that offset.
6. Paint Elements centered horizontally and vertically in its resulting caption rectangle, using the same caption font and foreground resolution as before.
7. Preserve the stock ComboBox response, closed selected-role painting, popup sizing and behavior, role routing, group response/cursor ownership, item widths, overflow, icon hover rectangles, and all current user-owned toolbar settings.
8. Record the implementation handoff in this draft and the follow-up overview.

## Non-goals

- No change to the ordinary shared caption gap or other toolbar outer padding.
- No ComboBox replacement or change to its closed control height, selected-role text, dropdown arrow, or popup.
- No independent Elements horizontal offset; the intended horizontal position is centered.
- No status, Script View, screenplay-core, persistence, pagination, or export changes.
- No agent-run validation.

## Structural acceptance for user review

1. Elements no longer uses a left-center caption anchor.
2. Action and Elements captions derive from equivalent nominal group-relative caption rectangles.
3. Actual ComboBox response height no longer controls the Elements caption's nominal center.
4. A zero-initialized Elements-only setting provides vertical optical adjustment without affecting ordinary Items.
5. Ordinary action caption geometry and all protected ComboBox/toolbar behavior remain unchanged.

## Durable records

- Update the toolbar caption contract in [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md:534) to require centered, common nominal caption alignment with a separately owned Elements optical adjustment.
- Update [`technical-context.md`](../../../.forge/blueprints/technical-context.md:28) to identify the shared caption-rectangle authority and the independence of nominal caption placement from stock ComboBox response height.
- Keep durable records free of work-order identifiers, implementation chronology, and numeric tuning values. Exact values and handoff evidence belong only in this planning directory.

## User-owned review requests

- Confirm Elements is horizontally centered beneath its dropdown.
- Compare Elements with New/Open/Save captions and confirm their vertical visual positions match at the default adjustment.
- If optical adjustment is needed, tune only the new Elements setting and confirm ordinary captions remain stationary.
- Exercise every selected-role label, narrow-width overflow, popup interaction, keyboard selection, and enabled commands.
- Run any desired workspace checks and record outcomes separately; no agent result is claimed.

## Implementation-submission handoff

- **Branch:** `dev/2026-11-05-gui-spacing-and-scroll` (the existing checkout; no branch or history operations performed).
- **Source change:** [`toolbar.rs`](../../../apps/scriptcraft/src/toolbar.rs:15) adds `TOOLBAR_ELEMENTS_CAPTION_VERTICAL_OFFSET = 0.0` and the shared `toolbar_caption_rect` helper. The helper starts the caption rectangle after the nominal control-slot height and existing shared gap. Action captions now use it; Elements uses the same nominal icon-control height, centers its caption in that rectangle, and applies only the Elements offset. The ordinary [`TOOLBAR_CONTROL_CAPTION_GAP`](../../../apps/scriptcraft/src/toolbar.rs:15) remains `1.0`; other user-adjusted toolbar values were left unchanged.
- **Resulting geometry:** With the current 36-point group and 18-point nominal control slot, both captions use the rectangle from 19 to 36 points below the group top (17 points high), centered horizontally in their group and vertically within the rectangle. The default Elements adjustment is neutral; only that caption's rectangle can be shifted by its setting. Actual ComboBox response height no longer determines the caption rectangle.
- **Protected behavior:** Stock ComboBox, selected-role painting, popup sizing and routing, response/cursor ownership, item widths, overflow, and icon hover geometry were not intentionally changed.
- **Evidence limitations:** No build, check, test, lint, formatting, manual verification, reviewer, validator, or user validation was run or claimed. This is an implementation submission only; user acceptance remains outstanding.

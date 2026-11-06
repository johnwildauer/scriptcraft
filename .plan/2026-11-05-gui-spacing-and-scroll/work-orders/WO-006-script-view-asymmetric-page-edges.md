# WO-006 — Independent Script View top and bottom margins

- **Status:** Implementation submitted for user validation; no validation or acceptance recorded
- **Initiative:** [`follow-up-refinements.md`](../follow-up-refinements.md)
- **Depends on:** Existing WO-003 implementation and subsequent user-owned source adjustments
- **Validation owner:** User; no agent-run builds, checks, tests, lint, formatting checks, manual verification, reviews, or independent validators

## Outcome

Remove the initial gap between the toolbar-side viewport edge and the first page while retaining independent bottom page-end padding.

At the top scroll position, the first page should begin flush with the usable Script View viewport. At the bottom, the last page should retain the current reviewed allowance beyond its boundary.

## Pre-implementation findings

- [`central_panel_frame()`](../../../apps/scriptcraft/src/script_view.rs:1476) already uses a margin-free frame.
- Before this submission, one shared page-edge setting applied 4 logical points to both content ends, making the remaining top gap intentional rather than a leftover fixed panel inset.
- Before this submission, prepaint zoom geometry used the same shared value in first-page origin and total content extent, so changing only the builder would make zoom geometry inconsistent.

## Scope

1. Replace the symmetric page-edge setting with [`SCRIPT_VIEW_TOP_PAGE_EDGE_PADDING`](../../../apps/scriptcraft/src/script_view.rs:17), initialized to zero, and [`SCRIPT_VIEW_BOTTOM_PAGE_EDGE_PADDING`](../../../apps/scriptcraft/src/script_view.rs:18), initialized to the current 4-point value.
2. Replace symmetric scroll-content margin construction with explicit independent top and bottom margins and zero horizontal content margins.
3. Update actual page allocation so the top margin precedes the first page and the bottom margin follows the existing final-page/trailing-gap content.
4. Update prepaint zoom geometry so first-page origin includes only the top setting and total content extent includes top plus bottom exactly once.
5. Preserve the existing page stride, inter-page gap, vertical item-spacing contribution, horizontal padding, page-local zoom scaling, outside-page displacement, clamping, actual page publication, pointer routing, caret reveal, and visible-page tracking.
6. Preserve the margin-free central frame and Script View-local zero-fade behavior. Do not change unrelated scroll areas.
7. Record the implementation handoff in this draft and the follow-up overview.

## Non-goals

- No change to the current bottom allowance other than moving it to an independent setting.
- No reintroduction of a fixed central-panel inset or scroll fade.
- No elastic scrolling, scroll snapping, or automatic page centering.
- No page gap, horizontal buffer, zoom limit, status, toolbar, screenplay-core, persistence, or export changes.
- No agent-run validation.

## Structural acceptance for user review

1. Top and bottom page-end spacing have independent source owners.
2. Initial/top-scroll first-page origin uses the zero top setting with no additional fixed workspace inset.
3. Bottom content extent retains the current allowance through its own setting.
4. Actual allocation and prepaint compensation use equivalent asymmetric geometry.
5. Existing scroll, zoom, page publication, input, and fade contracts remain intact.

## Durable records

- Update the Script View geometry requirements in [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md:137) to distinguish independent viewport-start and document-end spacing while retaining unobscured scrolling.
- Update [`technical-context.md`](../../../.forge/blueprints/technical-context.md:177) to identify independent content-end ownership and matching asymmetric actual/prepaint geometry.
- Keep durable records free of work-order identifiers, implementation chronology, and numeric spacing values. Exact settings and handoff evidence belong only in this planning directory.

## User-owned review requests

- At initial document display and after returning to the top, confirm the first page meets the toolbar-side viewport edge without the former blank gap.
- Scroll through the document and confirm there is still no fixed overlay or fuzzy band covering pages.
- At the bottom, confirm the existing allowance remains beyond the last page.
- Recheck horizontal overflow, zoom anchoring at all supported zoom paths, caret reveal, resize, page status, and pointer selection.
- Run any desired workspace checks and record outcomes separately; no agent result is claimed.

## Implementation-submission handoff

- **Branch:** `dev/2026-11-05-gui-spacing-and-scroll`; the existing checkout and all prior work, including WO-005, were preserved. No branch or history operations were performed.
- **Files changed for this submission:** [`script_view.rs`](../../../apps/scriptcraft/src/script_view.rs:17), [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md:198), [`technical-context.md`](../../../.forge/blueprints/technical-context.md:136), this draft, and [`follow-up-refinements.md`](../follow-up-refinements.md:3). WO-007 was not implemented or edited.
- **Exact source changes and values:** [`script_view.rs`](../../../apps/scriptcraft/src/script_view.rs:17) replaces the shared `SCRIPT_VIEW_PAGE_EDGE_PADDING` with `SCRIPT_VIEW_TOP_PAGE_EDGE_PADDING = 0.0` and `SCRIPT_VIEW_BOTTOM_PAGE_EDGE_PADDING = 4.0`. The scroll content margin now sets left/right to zero and uses those independent top/bottom values. Zoom compensation places the first page at the top setting and computes total vertical extent as page rows plus top and bottom once each. Page stride, page gap, horizontal padding, fades, page publication, and input behavior were not intentionally changed.
- **Resulting geometry:** At the initial/top scroll position, the first page has zero content margin above it. The bottom retains 4 logical points after the existing page-row/trailing-gap extent. Prepaint zoom geometry uses the same zero top origin and includes the preserved bottom allowance exactly once.
- **Evidence limitations:** No build, check, test, lint, formatting command, manual verification, reviewer, validator, or user validation was run or claimed. This is an implementation submission only; user validation and acceptance remain outstanding.

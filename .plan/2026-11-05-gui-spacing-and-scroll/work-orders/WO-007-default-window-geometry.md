# WO-007 — Shared startup and FDX default window geometry

- **Status:** Implementation submitted for user validation; no validation or acceptance recorded
- **Initiative:** [`follow-up-refinements.md`](../follow-up-refinements.md)
- **Depends on:** Existing native WindowState/TextState projection and save paths
- **Validation owner:** User; no agent-run builds, checks, tests, lint, formatting checks, manual verification, reviews, or independent validators

## Outcome

Start a fresh ScriptCraft process with the requested native outer position and inner dimensions, and keep generated/fallback FDX WindowState serialization consistent with those same defaults.

Use one existing typed default as the shared source rather than separate native and FDX numbers.

## Requested default representation

| Field | Value |
|---|---:|
| Height | 874 |
| Left | 0 |
| Mode | Normal |
| Top | 75 |
| Width | 870 |
| Scaling | 100 |
| Selection | 0,0 |
| ShowInvisibles | No |

The WindowState changes are Top 75 and Height 874; Left and Width retain their existing defaults. TextState and Mode already have the requested behavior and should not be redesigned.

## Baseline findings before implementation

- [`native_options()`](../../../apps/scriptcraft/src/main.rs:1358) specifies native chrome but no initial inner size or outer position, leaving startup geometry to platform-specific defaults.
- [`WindowState::DEFAULT`](../../../crates/scriptcraft-core/src/fdx.rs:277) currently uses Left 0, Top 95, Width 870, and Height 854.
- [`window_state_from_rects()`](../../../apps/scriptcraft/src/main.rs:1311) captures outer origin plus inner dimensions, and [`window_restore_commands()`](../../../apps/scriptcraft/src/main.rs:1331) restores that same mapping.
- [`TextState::DEFAULT`](../../../crates/scriptcraft-core/src/text_state.rs:18) already supplies Scaling 100 and caret 0,0; [`text_state_element()`](../../../crates/scriptcraft-core/src/fdx.rs:2633) emits Selection and ShowInvisibles No, while [`window_state_element()`](../../../crates/scriptcraft-core/src/fdx.rs:2619) emits Mode Normal.

## Scope

1. Update [`WindowState::DEFAULT`](../../../crates/scriptcraft-core/src/fdx.rs:277) to Left 0, Top 75, Width 870, and Height 874.
2. In [`native_options()`](../../../apps/scriptcraft/src/main.rs:1358), consume that typed default when constructing the native viewport.
3. Request initial inner dimensions through [`ViewportBuilder::with_inner_size()`](../../../apps/scriptcraft/src/main.rs:1360) using default Width/Height, matching the existing FDX capture/restore meaning.
4. Request initial outer position through [`ViewportBuilder::with_position()`](../../../apps/scriptcraft/src/main.rs:1360) using default Left/Top, again matching the existing mapping.
5. Preserve native decorations and all existing viewport options. Do not add a second independent set of startup geometry constants.
6. Preserve one-shot imported WindowState restore after Open. Imported geometry may replace the process default for that window exactly as before.
7. Preserve New-document behavior: creating a new document must not reposition or resize an already open native window.
8. Preserve live geometry capture and transactional Save/Save As. Actual reported geometry remains preferred; the updated default is only generated/fallback state when no actual snapshot exists.
9. Leave TextState defaults, zoom restoration, selection mapping, ShowInvisibles output, direct source-backed serialization, source preservation, and failure safety unchanged.
10. Record the implementation handoff in this draft and the follow-up overview.

## Non-goals

- No persisted application-level window preference or new FDX field.
- No guarantee that every native window manager accepts exact requested position or dimensions.
- No forced geometry reset on New, Save, or Save As.
- No change to imported WindowState validation or one-shot restore policy.
- No screenplay semantics, pagination, layout, export, toolbar, status, or Script View changes.
- No agent-run validation.

## Structural acceptance for user review

1. One typed default supplies both native startup requests and generated/fallback FDX geometry.
2. Width/Height map to native inner size and Left/Top map to native outer position, consistent with existing capture and restore.
3. The requested default values match the supplied WindowState representation; Mode and TextState retain their existing correct output.
4. Imported restore, New-document stability, latest-geometry capture, and transactional save behavior remain unchanged.
5. No duplicate native-only geometry source is introduced.

## Durable records

- Update the WindowState requirements in [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md:398) to state that fresh-process startup and generated/fallback FDX geometry share one default and use the existing outer-position/inner-size mapping.
- Update [`technical-context.md`](../../../.forge/blueprints/technical-context.md:178) with the same durable ownership boundary while retaining imported restore and live capture contracts.
- Keep durable records free of work-order identifiers, implementation chronology, and numeric geometry or zoom defaults. Exact values and handoff evidence belong only in this planning directory.

## User-owned review requests

- Launch a fresh process and inspect the initial native window position and inner dimensions.
- Create and save a new document, then inspect generated WindowState and TextState output. Confirm actual captured geometry correctly replaces defaults when available.
- Open documents with distinct saved WindowState values and confirm their one-shot restore still occurs; confirm New does not move or resize the open window.
- Check Save/Save As failure safety and confirm imported source preservation remains unchanged.
- Run any desired workspace checks and record outcomes separately; no agent result is claimed.

## Implementation-submission handoff

- **Branch:** `dev/2026-11-05-gui-spacing-and-scroll`; the existing checkout and all earlier work, including WO-005, WO-006, planning handoffs, and user-owned source adjustments, were preserved. No branch or history operations were performed.
- **Files changed for this submission:** [`fdx.rs`](../../../crates/scriptcraft-core/src/fdx.rs:277), [`main.rs`](../../../apps/scriptcraft/src/main.rs:1358), [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md:399), [`technical-context.md`](../../../.forge/blueprints/technical-context.md:184), this draft, and [`follow-up-refinements.md`](../follow-up-refinements.md:1).
- **Exact source changes and values:** `WindowState::DEFAULT` is the shared typed source with Left 0, Top 75, Width 870, and Height 874. `native_options()` now requests `with_position(left, top)` for the outer origin and `with_inner_size(width, height)` for inner dimensions from that same value. Native decorations and other viewport options remain intact.
- **Resulting contract:** Fresh-process native startup requests the shared outer position and inner size; generated/fallback FDX state uses the same default. Imported WindowState still uses its existing one-shot restore; New-document behavior, latest live geometry capture, and transactional Save/Save As paths were not changed. TextState, Mode Normal, and ShowInvisibles remain unchanged.
- **Evidence limitations:** No agent-run build, check, test, lint, formatting command, manual verification, independent review, validator, or user validation was performed. Platform acceptance of the requested startup geometry is not established. This handoff is for user review only; no validation or acceptance is claimed.

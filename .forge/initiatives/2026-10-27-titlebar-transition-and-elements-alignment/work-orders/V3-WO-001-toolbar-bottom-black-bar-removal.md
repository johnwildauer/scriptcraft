# Work Order: V3-WO-001 — Toolbar Bottom Black-Bar Removal

## Metadata

- **Status:** `complete`
- **Assignee:** Fresh desktop chrome-geometry implementer
- **Phase/order:** 1 of 2; first dependency
- **Initiative:** [`../titlebar-transition-and-elements-alignment-v3.md`](../titlebar-transition-and-elements-alignment-v3.md:1)
- **Generated filename:** `V3-WO-001-toolbar-bottom-black-bar-removal.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** pending fresh validation artifact `validation-V3-WO-001.md`
- **Fixes:** none
- **Implementer session/agent ID:** `implementer-fresh-v3-wo001-toolbar-bottom-2026-10-27T2005Z`
- **Session exception:** none

## Summary / one outcome

Remove the user's confirmed real black strip directly beneath the toolbar by correcting its actual app-owned geometry, fill, clipping, or physical-pixel coverage. The outcome is a deterministic chrome surface with no unexplained uncovered row and a mandatory user visual confirmation on the rebuilt running UI.

## Baseline

The user observed a real black bar a few pixels high immediately below the toolbar; it survived deleting the target folder and rebuilding from scratch. This is confirmed acceptance evidence, not a hypothesis and not a native-smoke limitation to document away.

## In scope

- Inspect [`update()`](../../../../apps/scriptcraft/src/main.rs:2163), [`chrome_frame()`](../../../../apps/scriptcraft/src/main.rs:2061), [`toolbar_content_height()`](../../../../apps/scriptcraft/src/main.rs:2095), [`toolbar()`](../../../../apps/scriptcraft/src/main.rs:1087), and central-panel placement.
- Reconcile panel/frame/toolbar row/bottom padding/central-panel rectangles, fill/stroke/separator, clip, full-size-content boundary, and logical-to-physical row rounding.
- Implement the smallest direct source correction that covers the actual app-owned strip without blind paint-over or unrelated layout redesign.
- Preserve the full-width title hit region, unified app-chrome fill, no stroke/separator, toolbar allocation, `TOOLBAR_ELEMENTS_WIDTH = 128.0`, cursor ownership, popup behavior, and document placement.

## Out of scope

- Transition metric policy, viewport animation control, `has_requested_resize`, desktop/work-area polling, global egui zoom, Elements behavior, semantic editing, or broad theme changes.
- Passing on “native evidence unavailable.” Only a proved outside-app ownership result may block the WO for user follow-up.

## Requirements

- **REQ-WRITING-LOOP-005:** “As an individual screenwriter, I want desktop commands in a consistent toolbar, so common project actions are discoverable without changing the semantic editing model.”
- **AC-WRITING-005.5:** “Given a toolbar width, Space shall consume one deterministic 72-point width, rendered normal Items shall consume their complete group width including label text measured at runtime with the active egui style/font, and Flexible Space Items shall divide the remaining width equally after the shared 4-point horizontal row-padding and 4-point inter-item-gap budget, and a narrow toolbar shall retain the leftmost fitting Items while exposing non-spacing Items through a far-right `>>` text menu. Action entries and Elements role choices in that menu shall remain executable without persisting toolbar layout state; a fully fitting composition shall keep its rightmost Item, including its label, inside the actual clipped toolbar content right edge after panel, frame, and viewport insets are applied. Items with an approved typed width override, including the Elements control, shall use that same resolved width for allocation and rendering without changing the shared padding, gap, flexible-space, overflow, or clipped-edge budget.”
- **AC-WRITING-005.6:** “Given the desktop app is running, a dedicated compact title strip shall display centered bold `ScriptCraft` placeholder text above the toolbar. A double-click shall request maximize/unmaximize only when the viewport's maximized state is known; unavailable or ignored state shall use a bounded no-op/native fallback rather than an unsafe or exact-restoration claim that the platform cannot support.”

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:83) — shared row, clip, edge, and 128-point Elements geometry.
- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:113) — unified title/toolbar surface, explicit fill, zero inset, and no separator stroke.
- [`technical-context.md`](../../../blueprints/technical-context.md:31) — presentation geometry remains above fixed semantic layout.

## Current source/API findings

- [`update()`](../../../../apps/scriptcraft/src/main.rs:2170) exact-heights one top panel and disables its separator, then places [`CentralPanel`](../../../../apps/scriptcraft/src/main.rs:2222) below it.
- [`chrome_frame()`](../../../../apps/scriptcraft/src/main.rs:2061) uses the toolbar chrome fill and `Stroke::NONE`; the existing source does not intentionally paint a black seam.
- [`toolbar_content_height()`](../../../../apps/scriptcraft/src/main.rs:2095) adds two interaction heights and two points of bottom padding, while [`toolbar()`](../../../../apps/scriptcraft/src/main.rs:1106) allocates the row and [`ui.add_space()`](../../../../apps/scriptcraft/src/main.rs:1143) adds the bottom padding.
- [`ChromeSurfaceLedger`](../../../../apps/scriptcraft/src/main.rs:92) and [`logical_y_to_physical_rows()`](../../../../apps/scriptcraft/src/main.rs:115) provide a starting ledger, but v2's deterministic continuity did not disprove the user's native pixel observation.
- eframe/egui `0.29.1` full-size-content/titlebar settings are native presentation hints, so ownership must be proved from app geometry/fill paths before any platform limitation is retained.

## Implementation guidance

1. Reproduce or instrument the issue and record panel, frame, title, toolbar-content, toolbar-row, bottom-padding, clip, central-panel, and physical pixel boundaries at representative scale factors. Identify which owner should cover each row immediately below the toolbar.
2. Check exact-height arithmetic, interaction height, row padding, bottom padding, frame fill, disabled separator, central-panel top, clip containment, and floor/ceil physical rounding. Do not assume the prior ledger proves pixel ownership.
3. If app-owned, apply one direct source correction at the responsible geometry/fill boundary and retain the existing surface contracts. If outside app ownership, exhaust app-owned paths, record the proof and exact configuration, and mark the WO `blocked` for user investigation rather than passing it.
4. Re-run geometry and edge tests after correction; verify no impact to title hit coverage, toolbar right edge, Elements width, or document response.

## Files and systems

- **Create:** focused geometry/coverage helpers or tests only if required.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1) and this work order/evidence during implementation.
- **Avoid changing:** core crates, requirements unless the durable contract changes, blueprints unless ownership is proven durable, v1/v2 artifacts, Elements/cursor/popup semantics, and unrelated toolbar allocation.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; focused chrome/coverage tests; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`.
- **Exact deterministic tests:** assert panel → title → toolbar-content → row → bottom-padding → panel-bottom contiguity; assert frame fill, zero stroke, disabled separator, clip containment, and central-panel top at the corrected chrome bottom; map fractional boundaries with floor/ceil physical-row conversion at at least 1x, 2x, and a fractional scale; assert the corrected coverage owns every row in the former strip; preserve right-edge containment and `TOOLBAR_ELEMENTS_WIDTH == 128.0`.
- **User visual verification checkpoint:** after implementation and rebuild from a clean target, the user runs the actual app and visually checks normal, maximized, and restored frames. The black strip must be absent immediately beneath the toolbar. The user records pass/fail; no-display or unavailable-native evidence cannot be recorded as pass.
- **Acceptance test:** Given the confirmed black strip baseline, when the corrected app is rebuilt and run, then the app-owned toolbar/chrome surface covers the intended bottom boundary with no black strip; if ownership is proven outside app ownership after all app paths are exhausted, then the WO is `blocked` with evidence and next action, never silently accepted.

## Dependencies, risks, and handoff

- **Dependencies:** v2 chrome ledger/API findings; no dependency on Elements cursor work. WO-002 consumes the corrected chrome geometry.
- **Risks:** native full-size-content inset, adjacent-fill mismatch, physical-pixel rounding, or a central-panel displacement may masquerade as a strip; a blind extension could hide content or alter hit testing.
- **Handoff:** provide the corrected geometry/coverage contract and test evidence to [`V3-WO-002-stable-titlebar-transition-metrics.md`](V3-WO-002-stable-titlebar-transition-metrics.md:1).

## Durable-document impact

- **Requirements impact:** `verify unchanged` — [`AC-WRITING-005.5`](../../../requirements/features/screenplay-writing-loop.md:115) and [`AC-WRITING-005.6`](../../../requirements/features/screenplay-writing-loop.md:131) already require the bounded toolbar/title geometry; update only if a new durable user-facing boundary is proven.
- **Blueprint impact:** `verify unchanged` by default — [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:113) already states the app-surface fill/no-separator contract; update only for a confirmed durable ownership correction.
- **Affected documents/IDs:** REQ-WRITING-LOOP-005 / AC-WRITING-005.5–005.6; desktop chrome surface contract.
- **Rationale and evidence:** inspect the actual implementation diff and visual/geometry evidence; a native limitation requires explicit blocked evidence, not a documentation pass.
- **Authorized documentation changes:** minimal evidence-backed surface ownership wording only; no pixel-equality or cross-platform native-chrome claim.

## Completion checklist

- [x] Source correction or explicit user-blocked ownership result is recorded.
- [x] Deterministic geometry/coverage tests and required checks have evidence.
- [ ] User visual verification of the rebuilt UI is recorded.
- [x] Durable-document classification matches the actual diff.
- [ ] Fresh validation session evidence is recorded; validator, not implementer, marks completion.

## Implementation evidence — 2026-10-27

- **Branch/base evidence:** Before source changes, `git status --short --branch` reported `master` at `6a6739c5263edee9896b32c00fc6f8470e35f3d6` with the four untracked v3 planning/review artifacts preserved. `git switch -c dev/2026-10-27-titlebar-transition-and-elements-alignment-v3` succeeded. After switching, `git branch --show-current` reported the required v3 branch and `git rev-parse HEAD` and `git rev-parse master` both returned `6a6739c5263edee9896b32c00fc6f8470e35f3d6`; `master` is an ancestor of the branch. No reset, checkout, commit, or destructive action was used.
- **Exact diagnosis:** The app-owned logical boundaries were contiguous: the shared top panel/frame owns title, toolbar content, row, and bottom padding through the panel bottom; `CentralPanel` begins at that same `y`. The installed egui 0.29 API shows that `CentralPanel::default()` paints `Frame::central_panel(style)` with `style.visuals.panel_fill`, while the toolbar uses `chrome_frame()` and `toolbar_chrome_color()` (`widgets.open.weak_bg_fill`). The former strip was therefore an app-owned adjacent-fill mismatch at the central-panel boundary, not a stroke or separator: the top frame has zero stroke/inset and the top separator is disabled. Physical floor/ceil conversion can overlap an adjacent boundary row at fractional scale, but cannot leave a logical gap.
- **Source correction:** Added `central_panel_frame()` and applied it to the existing `CentralPanel`. It retains egui's normal central-panel 8-point document inset and all allocation/clip/response behavior, while changing only the central surface fill to the exact toolbar/chrome fill. This makes both app-owned surfaces cover the boundary with one color without a blind overlay or toolbar-height change.
- **Deterministic evidence:** Added `chrome_fill_owns_former_strip_rows_at_each_representative_scale`, covering 1x, 1.5x, and 2x floor/ceil physical-row conversion, shared logical boundary, clip containment, and frame containment. The existing bottom-padding ledger and fractional physical-row tests remain green. The frame-fill test now asserts both top chrome and central boundary frames use the same explicit fill while the original `panel_fill` style remains unmutated. `TOOLBAR_ELEMENTS_WIDTH` remains `128.0`; title hit coverage, toolbar allocation, cursor/popup/document response, and viewport code were not changed.
- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1), this work order, and the v3 execution log. No requirements, blueprint, core crate, v1/v2 artifact, V3-WO-002 implementation, or validation artifact was created or changed.
- **Checks:** `cargo fmt --all -- --check`, focused chrome fill/ledger/frame tests, `cargo check --workspace`, `cargo test --workspace` (82 app tests, 45 core tests, integration/doc tests), `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` all passed. One initial new coverage assertion incorrectly required physical adjacency at a fractional boundary (`87` vs `86`); it was corrected to assert no uncovered row under floor/ceil expansion, then the focused tests and full required checks passed.
- **Native/user gate:** No native pixel claim is made in this implementation record. The required next action is a clean rebuild and user visual pass in normal, maximized, and restored frames. The user must record pass/fail before any fresh validator is dispatched; unavailable native evidence cannot be used as acceptance.
- **Next state:** `in-review`, awaiting the user's rebuilt-app visual verification and then a separate fresh validation session. This implementer did not create `validation-V3-WO-001.md` and did not mark the work order `complete`.

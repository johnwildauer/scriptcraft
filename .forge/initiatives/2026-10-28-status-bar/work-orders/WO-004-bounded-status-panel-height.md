# Work Order: WO-004 — Bounded Status Panel Height

## Metadata

- **Status:** `complete` — the separate, limited WO-004 alignment validation passed; this does not claim a comprehensive code review or independent rerun of implementation checks.
- **Assignee:** Separate fresh coding implementer, as explicitly directed by the user for this follow-on.
- **Phase/order:** Follow-on correction after the completed v1 status-bar work; one narrow work order.
- **Initiative:** [`status-bar-v2.md`](../status-bar-v2.md)
- **Generated filename:** `WO-004-bounded-status-panel-height.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-004.md`](../validations/validation-WO-004.md) — limited alignment result `PASS`; validator host dispatch ID is `01a1852b-403a-729b-bbcf-4b0afb023775` (actual host dispatch ID; no internal session/agent ID is inferred).
- **Fixes:** `none` — this is a new follow-on regression WO, not a fix for a failed validation.
- **Implementer host dispatch ID:** `01a18521-c1ea-77e1-a663-4aaf5138648e` (actual host dispatch ID supplied for the implementer; no internal session/agent ID is available or inferred). The investigation host ID is distinct.
- **Session-separation exception:** `none` — this is a separate fresh coding implementer; no agents were spawned by this implementer.
- **User execution request / gate exception:** The user explicitly directed this separate coding agent to implement WO-004 and explicitly overrode the usual pre-execution independent design-review and separate human-approval gate for this follow-on. Neither a WO-004 design review nor separate human design approval occurred; this exception is not represented as either.
- **Validation boundary:** The recorded `PASS` is limited to alignment with this work order and absence of clearly unrelated changes. It is not a comprehensive code review, implementation-effectiveness assessment, or independently rerun test/smoke result.

## Summary

At normal widths the status bar's bottom panel increases its height on successive egui frames until it consumes the editor viewport and approaches the toolbar. A focused reproduction and one-variable ablation identify the normal-width page column's full-available-height center-aligned child as the direct sizing trigger. Correct that bounded allocation and add a repeated-frame regression while preserving normal and compact status behavior.

## Observed bug and evidenced cause

- [`ScriptCraftApp::update()`](../../../../apps/scriptcraft/src/main.rs:1132) composes the toolbar, bottom status `TopBottomPanel`, then central editor; the bottom panel call is at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1172).
- [`status_bar::show()`](../../../../apps/scriptcraft/src/status_bar.rs:506) enters the 3-column branch at normal widths. The first column calls [`Ui::with_layout()`](../../../../apps/scriptcraft/src/status_bar.rs:551) with `Layout::left_to_right(egui::Align::Center)` and renders the visible-page label.
- In pinned egui 0.29.1, `Ui::with_layout` creates a child using the caller's current available rectangle; the center alignment is therefore relative to the status column's full available height. `Ui::columns` collects child minimum heights into the parent layout size, and `TopBottomPanel` stores the measured response rectangle as panel state for the next frame. The oversized center-aligned child minimum therefore feeds the current panel height into the next frame's measured minimum, producing positive height feedback.
- **Diagnostic reproduction:** Inline headless Rust diagnostic compiled against the checkout's pinned `target/debug/deps/libegui-8ae6bdcae0284441.rlib`; no app/repository source was changed and the temporary executable was removed. With the normal-width panel/columns/alignment pattern at 1200 × 820, status height measured 71 px on frame 1, 260 px on frame 10, 470 px on frame 20, 680 px on frame 30, and 803 px on frame 40. With the compact vertical/wrapped branch at 480 × 820, height remained 78 px at frames 1, 10, 20, 30 and 40.
- **One-variable ablation:** At 1200 × 820, center alignment grew 71 → 92 → 134 → 218 → 302 px at frames 1, 2, 4, 8 and 12; changing only the column alignment to `Align::Min` kept height at 71 px through frame 12, as did direct page-label layout. This isolates the normal-width center-aligned child allocation as the trigger.
- The diagnostic is a focused minimal egui reproduction of the production sizing path, not a complete `ScriptCraftApp` render or a native-window observation. The native UI symptom has not been directly observed in this tool session; desktop smoke remains unavailable/unperformed and is not claimed as passing.

## In scope

- Correct the status bar's normal-width page-column allocation so its child is content-sized or otherwise bounded and cannot create a height-dependent feedback loop through the bottom panel's saved response rectangle.
- Preserve the intended one-row three-region layout and sensible page-label alignment without forcing the label to the top if a bounded alignment can retain its current vertical presentation.
- Add a deterministic rendered egui regression that invokes the actual status renderer within the same toolbar → bottom panel → central panel ordering and measures the bottom panel rectangle on repeated frames.
- Cover both normal width (1200 × 820 logical points) and compact width (480 × 820 logical points), with fixed content and no pointer resizing/input.
- Preserve current status/error/path/diagnostic, visible-page, zoom, accessibility, toolbar, editor, page geometry, source-map, PDF and persistence behavior.
- Apply the authorized narrow additive requirements update for stable bounded status-panel height; verify the desktop blueprint unchanged.
- Record exact changed paths, all check attempts/results, comparable diff evidence, durable-document reconciliation, and honest native-smoke limitation.

## Out of scope

- Any new status item, content/action, breakpoint, zoom/page behavior, global UI scale, toolbar/editor rework, theme change, or broad status-bar redesign.
- A global or arbitrary hard panel-height cap that clips wrapped content or hides required status/error/diagnostic controls instead of correcting the sizing cause.
- Changes to core, fixed screenplay pagination, source mapping, export/PDF, project schema, persistence, dirty/undo state, Cargo dependencies, manifest or unrelated files.
- Claiming native visual/input success based on the focused headless rendering diagnostic or a process launch alone.

## Requirements

- **REQ-WRITING-LOOP-006:** Desktop status bar and visible page, [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:142).
- **AC-WRITING-006.1–006.3:** Preserve item interaction/accessibility, truthful current session/path/frame/operation status, visible errors and privacy-safe diagnostics; exact criteria are in [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:151).
- **AC-WRITING-006.4:** “At narrow widths, a deterministic bounded fallback shall retain access to visible-page state, clean/unsaved state, frames, path, operation/error status, and diagnostics.” See [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:164). Preserve this criterion and the existing compact behavior.
- **Authorized additive criterion:** Implemented as [`AC-WRITING-006.8`](../../../requirements/features/screenplay-writing-loop.md): at fixed viewport/content, the panel height range after frame 1 is ≤1 point over 60 egui frames at normal and compact widths, without monotonic growth or obscuring toolbar/editor/status content.
- Existing page/zoom criteria, including [`AC-WRITING-006.7`](../../../requirements/features/screenplay-writing-loop.md:180), remain unchanged.

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:126) assigns status presentation to the desktop and describes the three regions and narrow wrapped fallback. Verify unchanged: no ownership, architecture, runtime, or core boundary changes are authorized.

## Implementation plan

1. Re-read the current normal-width status rendering and pinned egui sizing behavior; verify the direct trigger against the actual implementation before changing source.
2. Make the smallest local layout correction in [`apps/scriptcraft/src/status_bar.rs`](../../../../apps/scriptcraft/src/status_bar.rs:506) so the page-label child cannot claim a height derived from the full available panel height. Preserve the intended horizontal page label and three-region composition; do not solve it by clipping with a global panel cap.
3. Add a regression that renders the actual `status_bar::show()` content through the bottom `TopBottomPanel` in the app's toolbar/status/central order for 60 repeated frames at 1200 × 820 and 480 × 820. Record each status response rectangle (or enough samples to prove the range and trend) and assert stability after the first frame.
4. Confirm critical normal and compact status items remain emitted/reachable, including the existing narrow zoom/page/error content checks. Preserve all other existing behavior.
5. Add only the authorized additive stable-height criterion to [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:142), verify [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:126) remains accurate, run required checks, inspect the exact diff/path inventory, and report native smoke availability honestly.

## Files and systems

- **Create/update:** [`apps/scriptcraft/src/status_bar.rs`](../../../../apps/scriptcraft/src/status_bar.rs) for the narrow layout correction and rendered repeated-frame regression; [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md) for one approved additive stable-height criterion; this WO and the initiative execution log for implementation evidence.
- **Verify unchanged:** [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md).
- **Avoid changing:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs) unless implementation evidence demonstrates the owning layout boundary is not in `status_bar.rs`; if so, stop and request a scoped plan/WO amendment before broadening. Avoid toolbar files, core, manifests, lockfile, exports and all unrelated paths.

## Validation and E2E acceptance tests

- **Focused regression:** Actual egui rendering with unchanged viewport and status content for at least 60 frames at normal 1200 × 820 and compact 480 × 820 widths. Capture the bottom `TopBottomPanel` response rect per frame.
- **Normal-width measurable acceptance:** After frame 1, status-panel `max(height) - min(height)` across frames 2–60 is `≤ 1.0` logical point; no monotonically increasing sequence occurs; the panel does not reach/cover the toolbar or consume the editor viewport.
- **Compact-width measurable acceptance:** After frame 1, panel-height range across frames 2–60 is `≤ 1.0` logical point; the compact layout continues to expose page, clean/unsaved, frames, path, operation/error, diagnostics and zoom without progressive panel growth.
- **Preservation:** Existing `status_bar_`, `visible_page_`, `zoom_` and `toolbar_` focused behavior remains green; all other existing app/workspace tests pass. Do not change fixed layout/page/source/PDF/project data or session state.
- **Project-policy commands:** Required checks are `cargo fmt --all -- --check`; `cargo test -p scriptcraft status_bar_`; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; and `git diff --check`. Exact outcomes are recorded in the implementation evidence and execution log below.
- **Manual/external:** Recommended native app smoke at normal and narrow widths to observe panel height and content reachability. The current manifest has no desktop-smoke hard-stop; if unavailable, record unavailable and do not claim visual success. A bounded `cargo run` alone is not smoke evidence.
- **Acceptance test:** Given a fixed viewport and unchanged status content, when egui renders 60 consecutive frames at normal or compact width, then the bottom status panel remains bounded within the stated 1-point frame range, leaves the toolbar/editor visible, and retains the appropriate normal/compact status controls and content.

## Durable-document impact

- **Requirements impact:** `update` — added only AC-WRITING-006.8 for measurable stable panel height; existing AC-WRITING-006.1–006.7 wording and status/page/zoom semantics are unchanged.
- **Blueprint impact:** `verify unchanged` — [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md). It already places status-bar responsibility in the desktop and describes the three-region/narrow wrapped layout; this fix changes local egui sizing, not component ownership or architecture.
- **Rationale/evidence:** The existing requirement covered narrow access but not repeated-frame height stability. Actual-renderer repeated-frame evidence confirmed user-visible normal-width growth; the additive measurable criterion now captures the bounded fix.
- **Authorized documentation changes:** Only the one additive stable-height requirement criterion if approved and implemented; no blueprint changes unless evidence reveals a different durable architecture.

## Completion checklist

- [x] The normal-width height feedback is removed by bounding the page-column child to sibling content height, without a global clipping cap.
- [x] A direct actual-renderer 60-frame regression passes at both 1200 × 820 and 480 × 820; height range after frame 1 is ≤1 logical point, growth is rejected, toolbar/editor visibility is asserted, and status controls/content are checked.
- [x] Required normal/narrow content and existing status/page/zoom/toolbar/editor contracts remain covered by focused regressions.
- [x] AC-WRITING-006.8 is updated; the desktop blueprint is verified unchanged.
- [x] Required project-policy checks are recorded below; final `git diff --check` and complete worktree inventory are pending recording in the execution log.
- [x] Native smoke is reported as unavailable/not performed, not inferred from headless evidence.
- [x] Separate limited alignment validation is recorded in [`validation-WO-004.md`](../validations/validation-WO-004.md); no broader review or independent test rerun is claimed.
- [x] The user's explicit execution-gate exception is recorded; no design-review pass or separate human design approval is claimed or fabricated.

## Implementation evidence (2026-10-29)

- **Authorization/state:** The user explicitly directed this separate fresh coding agent to execute WO-004 and directly overrode the usual design-review/separate-human-approval workflow for this follow-on. Neither independent design review nor separate human design approval occurred. Actual implementer host dispatch ID is `01a18521-c1ea-77e1-a663-4aaf5138648e`; no internal session ID is inferred. WO-004 is now `complete` after the separate limited alignment `PASS` recorded below.
- **Changed paths:** `apps/scriptcraft/src/status_bar.rs`; `.forge/requirements/features/screenplay-writing-loop.md` (only additive AC-WRITING-006.8); `.forge/initiatives/2026-10-28-status-bar/status-bar-v2.md`; this WO; `.forge/initiatives/2026-10-28-status-bar/execution-log.md`. No other path was authorized or changed; the desktop blueprint was verified unchanged.
- **Cause/fix:** The normal-width page column inherited the full available status-panel height through its center-aligned child. A direct actual-renderer reproduction grew from 57 logical points on frame 1 through 78, 99, 120, … to 809 by frame 37, remaining at 809 through frame 60. The renderer now uses the measured content-row heights of the middle and zoom columns to bound the page child's allocation, retaining center alignment and the three-region layout.
- **Rendered regression:** `status_bar_panel_height_stays_bounded_across_repeated_rendered_frames` invokes the actual `show()` renderer in toolbar → bottom status → central editor order for 60 unchanged-content frames at each target viewport; it measures each bottom-panel response height, enforces ≤1-point range after frame 1 and rejects monotonic growth, asserts toolbar separation/editor visibility, and checks page/state/path/operation/error/diagnostics/zoom labels. The pre-fix test reproduced and failed on normal-width growth; after the fix it passed at both widths.
- **Attempts and focused checks:** `cargo test -p scriptcraft status_bar_panel_height_stays_bounded_across_repeated_rendered_frames -- --exact` exited 0 but selected zero tests; the corrected filter ran the test and exited 101 with the growth evidence above. The first `cargo fmt --all -- --check` failed on rustfmt-only layout; `cargo fmt --all` repaired it. Final `cargo fmt --all -- --check` passed. `cargo test -p scriptcraft status_bar_`: 14 passed; `cargo test -p scriptcraft visible_page_`: 4 passed; `cargo test -p scriptcraft zoom_`: 10 passed; `cargo test -p scriptcraft toolbar_`: 28 passed; all exited 0.
- **Project-policy checks:** `cargo check --workspace` passed; `cargo test --workspace` passed (187 total: 105 app, 45 core unit, 37 integration; 0 failed, 0 doc tests); `cargo build --workspace` passed; `cargo clippy --workspace --all-targets -- -D warnings` passed. Final `git diff --check` and path inventory are recorded in the execution log after documentation updates.
- **Smoke/validation:** Native desktop visual/input smoke was unavailable/not performed. No app-launch-only or headless result is presented as native smoke. The limited alignment validation is recorded separately; no comprehensive review or independent test rerun is claimed.

## Limited alignment validation reconciliation — 2026-10-29

- **Validation artifact/result:** [`validation-WO-004.md`](../validations/validation-WO-004.md) records `PASS` for directional alignment and no clearly unrelated changes, comparing the inspected worktree against baseline `58ab67ba341dbe94ca5d1fa7d3ed9b2bb8c07cd`, including the untracked v2 and WO documents.
- **Validator identity:** Actual validator host dispatch ID is `01a1852b-403a-729b-bbcf-4b0afb023775`; no internal session/agent ID is inferred.
- **State transition:** WO-004 `in-review` → `complete` on this explicitly requested limited alignment result. Implementer host dispatch ID is `01a18521-c1ea-77e1-a663-4aaf5138648e`.
- **Limitations:** No comprehensive code/effectiveness review, test rerun, desktop smoke, WO-004 design review, or separate human design approval is claimed. Reported implementation checks remain implementer-reported only.

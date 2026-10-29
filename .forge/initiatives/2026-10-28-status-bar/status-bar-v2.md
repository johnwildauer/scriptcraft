# Initiative: Status Bar — v2 (Bounded Panel Height)

## Initiative metadata

- **Phase:** `execute`
- **Status:** `active` — WO-004 is complete after a separate limited alignment validation passed; this is not a comprehensive implementation review or independent test rerun.
- **Owner/orchestrator:** Forge orchestrator
- **Created/updated:** 2026-10-29
- **Authoritative version:** `v2` for this follow-on correction; v1 remains preserved as the record for the completed original status-bar scope.
- **Predecessor:** [`status-bar-v1.md`](status-bar-v1.md)
- **Initiative mode:** `standard`
- **Mode requested by:** `user`
- **Mode confirmation:** Standard initiative mode remains recorded; the user explicitly requested implementation in a separate fresh coding session and directly overrode the usual pre-execution design-review/separate-human-approval gate for this follow-on. No rapid path was requested.
- **Base/default branch:** `master` at `83621f34f5db17b3060b7c28fce795decd903581` (reconfirmed during this task).
- **Initiative branch:** `dev/2026-10-28-status-bar` (already active; no branch operation performed).
- **Branch policy:** `required`; the existing initiative branch satisfies the branch requirement for this follow-on.
- **Investigation HEAD:** `58ab67ba341dbe94ca5d1fa7d3ed9b2bb8c07cd`.
- **Direct-translation confirmation:** `requested and explicit` — the user directly requested the investigation, v2 plan, WO-004, and then implementation by a separate coding agent.
- **User execution request / gate exception:** The user explicitly directed this separate fresh coding agent to execute WO-004 and explicitly stated that this instruction overrides the usual pre-execution independent-design-review and separate-human-approval workflow. Neither a WO-004 design review nor separate human design approval occurred; this execution is not represented as either. Actual implementer host dispatch ID `01a18521-c1ea-77e1-a663-4aaf5138648e`; no internal session ID is available or inferred.

## Standard artifact locations

- **Authoritative plan:** this file.
- **Predecessor plan:** [`status-bar-v1.md`](status-bar-v1.md) — preserved; its three original work-order outcomes and validation history are not reopened by this narrow follow-on.
- **Work order:** [`WO-004-bounded-status-panel-height.md`](work-orders/WO-004-bounded-status-panel-height.md) — `complete` after the limited alignment result recorded in [`validations/validation-WO-004.md`](validations/validation-WO-004.md).
- **Review/validation/log artifacts:** The planning snapshot above is historical. WO-004 execution and limited alignment result are recorded in [`execution-log.md`](execution-log.md) and [`validations/validation-WO-004.md`](validations/validation-WO-004.md); no separate design review or human approval occurred.

## Outcome and boundary

- **Parent outcome:** The desktop status bar delivered by v1, especially the item-based three-region layout and narrow fallback in [`REQ-WRITING-LOOP-006`](../../requirements/features/screenplay-writing-loop.md:142) and the desktop status-bar ownership in [`scriptcraft-desktop.md`](../../blueprints/containers/scriptcraft-desktop.md:126).
- **Single observable outcome:** At a fixed viewport size, the bottom status panel reaches a bounded height and remains stable across repeated egui frames at normal and narrow widths; it does not progressively grow upward to cover the editor/toolbar.
- **In scope:** One focused correction to the normal-width status-bar sizing feedback and a rendered/headless repeated-frame regression for both the normal and compact status layouts. Preserve all established v1 status, page, zoom, accessibility, toolbar, editor and core behavior.
- **Out of scope:** Reworking status content, breakpoint, page/zoom behavior, toolbar/editor layout, global egui scaling, fixed screenplay geometry, persistence, core behavior, or unrelated panels; native UI redesign; any broad panel-height cap that clips content instead of correcting the expanding allocation.
- **Split assessment:** Keep as one narrowly bounded WO-004 under the existing initiative directory; no new independent feature is needed.

## Investigation findings

### Observed source path

- [`ScriptCraftApp::update()`](../../../apps/scriptcraft/src/main.rs:1132) increments the frame counter, renders the fixed-height toolbar, composes the bottom `TopBottomPanel` at [`main.rs`](../../../apps/scriptcraft/src/main.rs:1172), then renders the central editor at [`main.rs`](../../../apps/scriptcraft/src/main.rs:1184). The status `TopBottomPanel` uses egui 0.29.1's default non-resizable behavior; no status `.resizable(true)` is present.
- [`status_bar::show()`](../../../apps/scriptcraft/src/status_bar.rs:506) chooses the compact vertical/wrapped layout below 720 points, otherwise enters `ui.columns(3, ...)`. In that normal-width branch, the page column runs [`Ui::with_layout()`](../../../apps/scriptcraft/src/status_bar.rs:551) using `Layout::left_to_right(Align::Center)` over the column's full available height.
- Pinned egui 0.29.1 `Ui::with_layout` allocates a child UI from the caller's current available rectangle, and `Ui::columns` advances its parent using the maximum column `min_size().y`. `TopBottomPanel` measures/stores the returned panel response rectangle as `PanelState` for the next frame. The centered child therefore introduces a height-dependent minimum into the status panel; the next frame receives that taller panel as its available column area, feeding back into the measured minimum. This is the evidenced normal-width growth path.
- The compact branch in [`status_bar::show()`](../../../apps/scriptcraft/src/status_bar.rs:520) uses `ui.vertical` and wrapped rows instead of the normal branch's center-aligned column child; it did not expand in the repeated-frame reproduction.

### Candidate sources considered and narrowed

1. A user resize gesture or persistent panel-resize state — not supported as the cause: the application does not enable panel resizing, and the headless reproduction used no pointer input.
2. Long path/error labels wrapping into additional rows — not sufficient: the content was held fixed in the reproduction, yet height increased every frame.
3. Frame counters or other dynamic diagnostic text widening over time — not sufficient: fixed label text reproduced the growth.
4. The collapsed Diagnostics disclosure/animation — not supported: it remained closed and unchanged; the compact comparison retained a stable height.
5. Toolbar/central-panel ordering or general bottom-panel behavior — not sufficient: the same panel order and central panel remained in the alignment ablation, which stopped the growth.
6. Normal-width three-column layout with a center-aligned child spanning the column's full available height — **confirmed as the direct trigger in a focused egui 0.29.1 reproduction**; removing only the center alignment eliminated growth.
7. Compact breakpoint/wrapped layout — **ruled out for the observed repeated-frame expansion in the tested narrow case**; it remained stable.

### Reproduction, ablation, and limitations

- Compiled an inline, temporary Rust diagnostic against the already-built pinned `egui 0.29.1` rlib and ran 40 egui frames at each width; no source or persistent diagnostic file was added to the repository, and the temporary executable was removed.
- Normal-width reproduction at 1200 × 820 logical points: status height was 71 px on frame 1, 260 px on frame 10, 470 px on frame 20, 680 px on frame 30, and 803 px on frame 40 (the toolbar-to-editor space was consumed).
- Compact-width reproduction at 480 × 820: 78 px on frame 1 and 78 px at frames 10, 20, 30 and 40.
- One-variable ablation at 1200 × 820 for 12 frames: the exact page-column center alignment grew from 71 px (frame 1) to 92 (2), 134 (4), 218 (8), and 302 (12); replacing only `Align::Center` with `Align::Min` stayed at 71 px, as did direct page-label layout. This confirms the center-aligned child allocation as the growth trigger in the reproduction.
- The diagnostic reproduces the actual egui panel/columns/alignment sizing pattern and is linked to the matching production call sites, but is a focused minimal reproduction, not an invocation of the entire `ScriptCraftApp` or a native-window observation. Native desktop smoke was not performed and is not claimed. The manifest does not configure desktop smoke as a hard-stop gate.
- The initial shell attempt to use `rg` failed because ripgrep is not installed (`/bin/sh: rg: command not found`); subsequent source inspection used `find`, `grep`, and `sed` successfully. This caused no repository change.

## Proposed correction and work-order plan

- Keep the repair inside the normal-width page column: remove or bound the full-available-height center-aligned child sizing so the returned child/column minimum is content-sized, while retaining the intended one-row page indicator alignment and the existing three-region layout. Do not mask the issue with a panel height cap that clips or hides status content.
- Add a regression using the actual status renderer inside the same top/bottom/central egui panel ordering. At fixed 1200 × 820 and compact 480 × 820 viewports, render at least 60 consecutive frames and measure the bottom panel response rectangle each frame.
- WO-004 owns the source correction, the repeated-frame regression, and one narrow addition to the status-bar requirement. No dependency on or modification to the completed v1 work orders is proposed.
- Required preserved behavior includes page/clean/frames/path/operation/error/diagnostic reachability, zoom controls and accessibility at normal width, and the existing compact behavior at narrow width.

## Acceptance boundary

- On an unchanged normal 1200 × 820 egui viewport, after the first layout frame, the measured status-panel height has no monotonic per-frame increase and its maximum-minus-minimum across frames 2–60 is at most 1 logical point; the bottom panel never consumes the editor area or reaches the toolbar.
- On an unchanged compact 480 × 820 viewport, the panel likewise remains bounded across frames 2–60 within 1 logical point of its stable measured height; required status/page/zoom/error content remains reachable and no expanding panel obscures the editor.
- Existing toolbar, page-currentness, zoom, error, keyboard/accessibility, editor, fixed pagination/source-map/PDF and saved-project behavior remain unchanged.
- Automated output establishes panel rects through repeated egui frames. Native visual/input smoke is recommended when available but is not a configured hard-stop; absence must be recorded as unavailable, not passed.

## Requirements and blueprint references

- **Requirements:** [`screenplay-writing-loop.md`](../../requirements/features/screenplay-writing-loop.md), [`REQ-WRITING-LOOP-006`](../../requirements/features/screenplay-writing-loop.md:142), especially narrow fallback [`AC-WRITING-006.4`](../../requirements/features/screenplay-writing-loop.md:164) and status accessibility/content criteria [`AC-WRITING-006.1`](../../requirements/features/screenplay-writing-loop.md:151)–006.3. WO-004 authorizes a narrowly scoped additive criterion for stable bounded panel height under repeated fixed-viewport frames; no requirement file is changed in this planning task.
- **Blueprint:** [`scriptcraft-desktop.md`](../../blueprints/containers/scriptcraft-desktop.md), especially the status-bar ownership and wrapped compact layout contract at [`scriptcraft-desktop.md`](../../blueprints/containers/scriptcraft-desktop.md:126). WO-004 proposes `verify unchanged`; the desktop/component ownership boundary does not change.
- **Implementation evidence:** [`ScriptCraftApp::update()`](../../../apps/scriptcraft/src/main.rs:1132), [`status_bar::show()`](../../../apps/scriptcraft/src/status_bar.rs:506), and the normal-width center-aligned page column at [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs:551).

## Durable-document impact and policy gates

- **Requirements:** Planned `update` only for an additive, measurable stable-height criterion attached to REQ-WRITING-LOOP-006; implementation must apply only the authorized narrow change after the design gate.
- **Blueprint:** Planned `verify unchanged`; preserve the desktop ownership, three-region composition and narrow wrapped fallback.
- **Required project checks for implementation:** `cargo fmt --all -- --check`; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`; plus focused `cargo test -p scriptcraft status_bar_` covering the new repeated-frame regression.
- **Desktop smoke:** Recommended and explicitly non-mandatory under the current manifest. A run/liveness check alone is not rendered-window evidence.
- **Review/approval gate exception:** The manifest normally requires independent design review and separate human approval before execution. The user's explicit follow-on instruction authorized WO-004 execution as an exception; no independent design review pass or separate human approval occurred. This is recorded as the actual user-directed exception, not as satisfaction of the ordinary gate.

## Planning state and evidence

- **User authorization:** The user explicitly requested investigation, v2 plan, WO-004, and then implementation by a separate fresh coding agent. The follow-on user instruction directly overrides the repository's ordinary pre-execution review/approval workflow for this WO. No independent WO-004 design review or separate human design approval is claimed.
- **Git preflight:** Before edits, `git status --short --branch` returned `## dev/2026-10-28-status-bar`; the staged and tracked-unstaged diffs were empty; `HEAD` was `58ab67ba341dbe94ca5d1fa7d3ed9b2bb8c07cd`, and `master` was `83621f34f5db17b3060b7c28fce795decd903581`.
- **Changed artifact boundary:** This planning task creates only this v2 plan and [`WO-004-bounded-status-panel-height.md`](work-orders/WO-004-bounded-status-panel-height.md). No source, test, requirement, blueprint, manifest, v1 plan, execution log, review, validation, branch, index or commit change is authorized here.
- **Planning snapshot history:** The planning records above were authored before the follow-on implementation instruction. The current execution and exception are recorded below and supersede the planning-only/pending-gate state without erasing the historical planning evidence.

## Follow-on execution record — WO-004

- **Execution authorization:** User explicitly directed implementation by a separate fresh coding agent and explicitly overrode the usual design-review/separate-human-approval gate for this follow-on. No review pass or separate human approval occurred or is fabricated. No agents were spawned by this implementer.
- **State:** Initiative `execute` / `active`; WO-004 `complete` after the explicitly requested limited alignment validation. This does not imply broader review or test reruns.
- **Implementer identity:** Actual coding implementer host dispatch ID `01a18521-c1ea-77e1-a663-4aaf5138648e`; no internal session ID is available or inferred. The investigation host ID is distinct.
- **Implementation:** The normal-width page column now allocates a bounded row using the measured content height of its center/zoom sibling columns, preserving center alignment without inheriting the full available panel height. A direct repeated-render test invokes `status_bar::show()` within top toolbar → bottom status → central editor panel order at 1200 × 820 and 480 × 820 for 60 frames; it measures every bottom-panel response height, asserts ≤1-point range after frame 1, rejects monotonic growth, checks toolbar/editor visibility, and verifies required status labels and zoom remain emitted.
- **Regression evidence:** Before the layout fix, the new actual-renderer test failed at normal width: height grew from 57 px on frame 1 through 78, 99, 120… to 809 px by frame 37 and remained there through frame 60. After the fix, the same 60-frame test passes at both widths. Existing compact status, page, zoom, and toolbar focused tests pass.
- **Requirements and blueprint:** Added only authorized [`AC-WRITING-006.8`](../../requirements/features/screenplay-writing-loop.md) for 60-frame bounded-height stability. [`scriptcraft-desktop.md`](../../blueprints/containers/scriptcraft-desktop.md) was verified unchanged; no ownership or architecture changed.
- **Checks and limitations:** Implementation check outcomes, initial failed attempts, exact changed paths, current Git evidence, and native desktop smoke limitation are recorded in [`execution-log.md`](execution-log.md) and WO-004. Native window/input smoke was not available/performed and is not claimed. No stage, commit, reset, stash, merge, or validation dispatch was performed by the implementer.

## User-directed follow-on batch — WO-005 through WO-008

- **Batch state:** The one requested FAST gross-alignment design review returned `PASS`; at planning time all four follow-on WOs were `draft`. Current execution supersedes that snapshot: WO-005 is implementation-complete and `in-review` pending the user's single aggregate validation; its actual implementer host dispatch ID `01a185e7-91a8-74b8-ae3b-570218150a31` is reconciled. WO-006 implementation is complete and `in-review` for handoff; the sole aggregate check remains pending after WO-007 and WO-008. The review is a related continuation of v2, not a new initiative, and does not reopen WO-004 or the completed v1 work. See [`design-review-WO-005-through-WO-008.md`](reviews/design-review-WO-005-through-WO-008.md).
- **Outcome:** Apply four narrowly bounded desktop changes in sequence: remove selected status text and make status content clip horizontally without wrapping/growing the footer; route supported inward/outward native pinch/squeeze input to existing transient page-zoom state and establish the requested native/fallback UI scale; use the supplied Courier Prime family as the default document text face while preserving document/layout/export contracts and font ownership; remove only the visible project-title heading strip directly below the toolbar.
- **Sequence/dependencies:** [`WO-005-status-content-and-horizontal-clipping.md`](work-orders/WO-005-status-content-and-horizontal-clipping.md) → [`WO-006-native-pinch-squeeze-and-ui-scale.md`](work-orders/WO-006-native-pinch-squeeze-and-ui-scale.md) → [`WO-007-courier-prime-default-document-font.md`](work-orders/WO-007-courier-prime-default-document-font.md) → [`WO-008-remove-document-title-strip.md`](work-orders/WO-008-remove-document-title-strip.md). Use a distinct fresh coding implementer per WO, in this order; do not start a later WO before the prior implementation handoff.
- **Review/authorization:** The single requested FAST gross-alignment review passed with no gross blocker; its scope and decision are recorded in [`design-review-WO-005-through-WO-008.md`](reviews/design-review-WO-005-through-WO-008.md). Host review dispatch ID `01a185e5-73db-7287-a08d-cef519fbfd99` was supplied by the user and recorded; no separately given human design approval is claimed.
- **Validation exception:** The project manifest's default is fresh independent validation per WO. The user explicitly requests at most one narrow, light alignment validation over all four after the sequential implementations, and rules out per-WO validations, additional review agents, comprehensive effectiveness reviews, and reviewer test reruns. Record this as a user-directed reduced-confidence exception; make no validation claim until a real result exists. Required implementer checks remain in each WO.
- **Dispatch IDs/state:** WO-005 implementation is complete; actual implementer host dispatch ID `01a185e7-91a8-74b8-ae3b-570218150a31` was supplied by the user and reconciled in its WO and the execution log. WO-006 is `in-review`; actual implementer host dispatch ID `01a1860d-a80d-7069-8ad5-bbdb34b1492a` is reconciled in its WO and execution log. WO-007 is `in-review` after implementation and required checks; actual implementer host dispatch ID `01a1861e-76e0-7509-8b2c-c4e7dfc13367` was supplied by the user and reconciled in its WO and execution log. WO-008 is `in-progress`; its own host dispatch ID remains pending orchestrator reconciliation. The aggregate validator has not been dispatched and is not due until all four implementations are complete. These identifiers are host dispatch IDs only; no internal session/agent ID is inferred.
- **Protected baseline/work:** Preexisting modified [`execution-log.md`](execution-log.md), [`screenplay-writing-loop.md`](../../requirements/features/screenplay-writing-loop.md), and [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs), plus untracked v2/WO-004/WO-004 validation records and all user-supplied CourierPrime assets, are preserved. This planning task adds only these four WOs and appends this plan/log; it does not edit product source, requirements, blueprints, fonts, `.DS_Store`, WO-004, its validation, or historical evidence.
- **Policy boundary:** Each implementer runs the project-manifest Cargo gates, the WO's focused regression(s), and `git diff --check`; report native visual/input smoke as unavailable unless actually performed. No gesture, UI-scale, font-rendering, or cross-platform support claim may exceed pinned APIs or observed platform evidence.

## WO-005 implementation handoff — 2026-10-29

- **Implementation state:** WO-005 is `in-review` with implementation complete for sequential handoff to WO-006. The user-directed single aggregate validation remains pending after WO-005–WO-008; no independent validation or separate human design pass is claimed.
- **Scope/evidence:** The rendered status no longer displays cumulative frames, project path/not-saved text, or the startup in-memory demo placeholder. Clean/unsaved, operation/error, visible page, zoom, and Diagnostics remain. Status and Diagnostics text are clipped as one-line content; Diagnostics removal is deferred until pre-release. Real-renderer regressions cover repeated frames and changing long status/diagnostic content at 1200, 721, 720, 480, and 320 logical points.
- **Checks:** Focused status/page/zoom/toolbar tests and the required Cargo workspace checks are recorded in the WO-005 evidence and the appended execution-log entry. Native visual/input smoke was not performed.
- **Dispatch:** Review host dispatch `01a185e5-73db-7287-a08d-cef519fbfd99`; WO-005 implementer host dispatch `01a185e7-91a8-74b8-ae3b-570218150a31` (actual ID supplied by the user and reconciled; no internal session ID is inferred).

## WO-007 implementation handoff — 2026-10-29

- **State:** WO-007 is `in-review`; implementation and required checks are complete. The user's sole aggregate alignment check remains pending until WO-008 implementation completes; no per-WO validator or expanded review is claimed. WO-008 implementation is now authorized and in progress.
- **Dispatch reconciliation:** WO-006's actual implementer host dispatch ID `01a1860d-a80d-7069-8ad5-bbdb34b1492a` is reconciled in WO-006 and the execution log. WO-007's actual implementer host dispatch ID `01a1861e-76e0-7509-8b2c-c4e7dfc13367` was supplied by the user and reconciled in its WO and the execution log. WO-008's own host dispatch ID is pending orchestrator reconciliation. These are host dispatch IDs only; no internal session ID is available or inferred.
- **Implementation:** New projects now persist `Courier Prime`. The desktop embeds all supplied Courier Prime faces at build time, selects regular for default document text, and preserves explicit stored-family round trips and established legacy rendering. No weight/slant semantics are added. Fixed character-cell placement, page/source/caret/selection/IME geometry, layout caching, and viewport scaling remain unchanged. PDF retains its existing Type1 family mapping and does not embed the supplied TTF.
- **Durable documents:** Updated format rules, screenplay-core/feature/desktop blueprints, and technical context. The writing-loop requirement was inspected and verified unchanged. All supplied fonts, README, OFL 1.1/FAQ, and `.DS_Store` remain at their original paths and byte-identical; no user-owned font file was edited.
- **Checks:** Focused font selection/data/render tests, profile round-trip/default tests, and PDF mapping tests passed. `cargo fmt --all -- --check`, `cargo test -p scriptcraft` (114), `cargo test -p scriptcraft-core` (45 unit + 37 integration), `cargo check --workspace`, `cargo test --workspace` (196; 0 failed), `cargo build --workspace`, strict workspace Clippy, `git diff --check`, and a built-executable scan for all four byte-identical TTF payloads passed. Exact scope, asset hashes, and limitations are recorded in WO-007 and the execution log.
- **Limitations/next:** Native visual/font smoke was not performed. No PDF embedding or external PDF-viewer glyph-parity claim is made. A separate distribution package is not configured; downstream redistribution must include the supplied OFL notice. Proceed to the separately authorized WO-008 only after this implementation handoff; defer the one aggregate check until all four implementations are complete.

## WO-008 implementation handoff — 2026-10-29

- **Implementation state:** WO-008 is `in-review` with implementation and required checks complete. The user-directed sole aggregate alignment validation remains pending; no per-WO validation, expanded review, or separate human approval is claimed.
- **Dispatch:** WO-007 actual implementer host dispatch ID `01a1861e-76e0-7509-8b2c-c4e7dfc13367` was supplied by the user and reconciled in WO-007 and the execution log. WO-008's own host dispatch ID remains pending orchestrator reconciliation; no internal session ID is inferred.
- **Scope/evidence:** `apps/scriptcraft/src/main.rs` removes only the central-panel metadata heading and separator. The actual shared toolbar/status/central-panel render regression proves no metadata-heading shape or reserved title row, while retaining rendered toolbar/status and title-page content, persisted metadata, and JSON round-trip behavior. Existing named save/open and failed-open tests pass.
- **Durable documents:** Requirement `AC-WRITING-001.1` and the desktop blueprint were inspected and verified unchanged; neither documents nor requires the redundant strip.
- **Checks/limitations:** Focused rendered regression 1/1; app suite 115/115; format, workspace check, workspace tests (197: 115 app + 45 core unit + 37 integration), build, strict Clippy, and `git diff --check` passed. Initial formatter-only failure and its exact correction are preserved in WO-008. Native desktop visual/input smoke was not performed and is not claimed.
- **Preservation/next owner:** Prior dirty work, user changes, font assets/licenses, and non-WO-008 source/documentation remain preserved. No stage, commit, reset, stash, merge, or cleanup was performed. Orchestrator may perform only the authorized single narrow aggregate alignment check across WO-005–WO-008; until its actual result exists, all four remain in-review.

## Aggregate alignment result — WO-005 through WO-008

- **Current batch state:** WO-005, WO-006, WO-007, and WO-008 are `complete` for the user's one limited aggregate directional-alignment check only. The check returned `PASS`; it is not comprehensive code validation, and initiative status remains `execute` / `active`.
- **Source of truth:** [`validation-aggregate-WO-005-through-WO-008.md`](validations/validation-aggregate-WO-005-through-WO-008.md) records baseline, scope, decision, attributed implementer-reported checks, and limitations. Earlier pending/in-review statements above are historical handoff snapshots and are superseded only for this limited completion state.
- **Dispatch reconciliation:** WO-008 actual implementer host dispatch ID `01a18632-ede4-70b7-a5d3-2046acd2c618` is reconciled in WO-008 and the execution log; no internal session ID is inferred.
- **Aggregate validator dispatch reconciliation:** Actual host dispatch ID `01a1863f-1a25-76a1-be17-5d3e91808e96` is reconciled in the aggregate validation and execution log. It is a host dispatch ID only; no internal session ID is inferred.
- **Preservation/cadence:** This is the sole requested aggregate alignment artifact; no per-WO validation or extra review was created. WO-004 records, the single prior design review, user-supplied font files/licenses, and earlier records remain preserved. No tests were rerun and no native desktop smoke was performed.

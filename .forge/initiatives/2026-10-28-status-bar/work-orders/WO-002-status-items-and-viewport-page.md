# Work Order: WO-002 — Status Items and Viewport Page

## Metadata

- **Status:** `in-review` — WO-002 implementation and required automated checks are complete; independent validation remains pending in a separate fresh session.
- **Assignee:** Fresh dedicated desktop application implementer subtask (dispatched 2026-10-28 16:40 UTC)
- **Phase/order:** 2 of 3; depends on WO-001; precedes integrated zoom regression in WO-003
- **Initiative:** [`status-bar-v1.md`](../status-bar-v1.md)
- **Generated filename:** [`WO-002-status-items-and-viewport-page.md`](WO-002-status-items-and-viewport-page.md)
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`validation-WO-002.md`](../validations/validation-WO-002.md) (future artifact; not created)
- **Fixes:** none
- **Initial implementer host subtask/dispatch ID:** `01a1835e-24db-7021-9c56-2becaf971f08` (actual host dispatch ID; internal session/agent ID is unavailable and is not inferred).
- **Fresh continuation implementer host subtask/dispatch ID:** `01a1836c-02ef-730c-988d-5f3c52b27655` (actual host dispatch ID; internal session/agent ID is unavailable and is not inferred).
- **Session exception:** none

## Dispatch and start — 2026-10-28 16:40 UTC

- **Actor/role:** Fresh dedicated WO-002 implementer subtask.
- **Host subtask/dispatch ID:** `01a1835e-24db-7021-9c56-2becaf971f08` (actual host dispatch ID; internal session/agent ID is unavailable and is not inferred).
- **State transition:** `draft` → `in-progress`; initiative remains `execute` / `active`.
- **Authorization/dependency:** User authorized WO-002 after WO-001 completed with independent validation `pass` for commit `fde481db08ba058a1327e2d23abb9b056c2556c3`.
- **Preflight:** Rechecked `git branch --show-current`, `git rev-parse HEAD`, `git status --porcelain=v2 --branch --untracked-files=all`, `git diff --cached --name-status`, `git diff --name-status`, and `git ls-files --others --exclude-standard`. Branch and HEAD match the authorized baseline. The empty index and pre-existing modifications/untracked WO-001 documentation were identified and are protected; no unrelated paths were present.
- **Scope:** WO-002 only. WO-003 remains unstarted. No staging, commit, reset, stash, merge or branch operation is authorized here.
- **Result at dispatch:** Implementation is beginning; no source edits or validation results are claimed yet. See completion evidence below when recorded.

## Implementation attempt and mandatory-gate blocker — 2026-10-28 16:48 UTC

- **Actor/role:** Fresh dedicated WO-002 implementer subtask.
- **Host subtask/dispatch ID:** `01a1835e-24db-7021-9c56-2becaf971f08` (actual host dispatch ID; internal session/agent ID is unavailable and is not inferred).
- **State transition:** `in-progress` → `blocked`. This is not `in-review`: implementation evidence and every required gate are not complete, and the user instructed the implementer to stop if a mandatory gate fails.
- **Failure and exact result:** `cargo fmt --all && cargo test -p scriptcraft status_bar_` exited 101. Formatting completed before the test command began. The focused test binary ran 12 tests: 11 passed and 1 failed. `status_bar::tests::status_bar_diagnostic_projection_contains_only_categories_and_counters` failed at `apps/scriptcraft/src/status_bar.rs:592` because its assertion expected `layout-cache-hits=6`, while the diagnostic formatter's actual field is `cache-hits=6`. This failure was not waived or rerun; implementation and validation stopped at this mandatory focused-test failure.
- **Changes made before the stop:** `apps/scriptcraft/src/status_bar.rs` (new status-item capability model/rendering, state projection, visible-page selector/currentness tracker, and focused tests); `apps/scriptcraft/src/main.rs` (status composition, actual rendered-page/clip publication, invalidation/repaint wiring, and rendered viewport/currentness tests); [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md) (additive `REQ-WRITING-LOOP-006`); [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md) (desktop status/page ownership); this WO and the execution log (dispatch/evidence state). This list is the complete known WO-002 edit set at stop.
- **Protected pre-existing records:** The pre-existing post-commit updates to the execution log and WO-001 work order and untracked `validation-WO-001.md` were preserved. No WO-001 validation text was edited. No unrelated path, core file, manifest, lockfile, or WO-003 artifact was changed.
- **Acceptance evidence boundary:** Pure selector tests for overlap/ties, nearest gaps/ties, bounds, title-sheet total and invalid geometry passed within the failed focused run; currentness/state/item tests also passed in that run. The same run includes one failing diagnostic assertion as stated above. `visible_page_`, toolbar regression, rendered scrolling integration, all remaining required project commands and a final diff check were not run after the hard stop; their results are **not established**. No acceptance criterion is claimed complete from this partial run.
- **Required checks remaining / not run:** `cargo fmt --all -- --check`; `cargo test -p scriptcraft visible_page_`; `cargo test -p scriptcraft toolbar_`; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; and `git diff --check` were not run because the mandatory focused filter failed and the user required a stop. No fresh independent validation occurred.
- **Desktop smoke:** Not run; the current tool surface has no native-window/input automation. The manifest does not configure desktop smoke as a hard-stop gate, but that optional limitation does not waive the failed automated test.
- **Git boundary:** The initial pre-implementation state was `dev/2026-10-28-status-bar` / `fde481db08ba058a1327e2d23abb9b056c2556c3` / empty index. The continuation rechecked the live complete worktree status and confirmed the branch/HEAD, empty index, and only the authorized/pre-existing paths. No stage, commit, reset, stash, merge, or branch operation was performed.
- **State at the initial stop:** WO-002 was `blocked`; the initial implementer stopped after the failed focused test as then directed. The fresh continuation and subsequent state transition are recorded below.

## Fresh continuation and implementation completion — 2026-10-28 16:51–17:00 UTC

- **Actor/role:** Fresh independent WO-002 continuation implementer, distinct from the initial implementation dispatch above.
- **Initial implementer host dispatch ID:** `01a1835e-24db-7021-9c56-2becaf971f08` (actual host dispatch ID; no internal session/agent ID is claimed).
- **Continuation host dispatch ID:** `01a1836c-02ef-730c-988d-5f3c52b27655` (actual host dispatch ID; no internal session/agent ID is claimed). This is a fresh continuation of WO-002, not a validation-fix work order.
- **State transition:** `blocked` → `in-progress` under the user's explicit fresh-continuation instruction, then `in-progress` → `in-review` after completing implementation and required evidence. Initiative remains `execute` / `active`; WO-001 remains `complete`; WO-003 remains `draft` and undispatched.
- **Failure classification and process decision:** The original `status_bar_diagnostic_projection_contains_only_categories_and_counters` failure was an implementation-time required-test failure, not a validation finding. The governing fix-work-order rule in `05-validation.md` applies to a genuine hard-stop validation finding; no validation had occurred. Therefore no letter-suffixed fix WO was created. The original failure is retained above without recasting it as validation.
- **Diagnostic test resolution:** `DiagnosticProjection::safe_summary()` deliberately emits the concise counter key `cache-hits` for the `layout_cache_hits` counter. The test now asserts the actual summary key `cache-hits=6` while retaining category and privacy-exclusion assertions. This aligns the expectation with the approved requirement for safe diagnostics; it does not weaken or remove the test and does not change the formatter's product output.
- **Additional implementation/test repairs:** Added a deterministic compact status layout below 720 logical points while retaining viewport/clean/frame state in its first wrapped row and path/operation/error access in its second; added breakpoint tests. The wide layout composes separate left page, center document-state, and neutral right-reserved regions without introducing WO-003 controls. Repaired the rendered-scroll integration test: the original wheel-input fixture failed to demonstrate a later visible page, and adding a pointer-move alone did not satisfy its immediate assertion. The revised test places the pointer over the editor, sends the wheel event, waits up to 58 subsequent frames for current rendered geometry, asserts a later visible sheet, and confirms the caret remains unchanged. Added a deliberate dead-code annotation for the unused-but-required reusable interaction response type; the final check is warning-free.
- **All command attempts and outcomes:**
  - Initial dispatch: `cargo fmt --all && cargo test -p scriptcraft status_bar_` — exit 101; 11 passed, 1 failed because expected `layout-cache-hits=6`, actual formatter output `cache-hits=6`.
  - Continuation inspection command using `rg` — exit 127 because `rg` is not installed; equivalent `grep` inspection succeeded. This was an inspection-tool limitation, not a project gate.
  - `cargo test -p scriptcraft visible_page_` before the integration-test repair — exit 101; the scroll fixture's assertion that a later page was selected was not met. Adding a pointer-move alone still exited 101 at the same assertion. The fixture was then corrected to wait for current geometry after the scroll before asserting its selected page.
  - `cargo fmt --all -- --check && cargo test -p scriptcraft status_bar_ && cargo test -p scriptcraft visible_page_ && cargo test -p scriptcraft toolbar_` — exit 1 at formatting; rustfmt reported formatting differences in the just-edited integration test and compact-layout test. Ran `cargo fmt --all`.
  - `cargo fmt --all && cargo fmt --all -- --check && cargo test -p scriptcraft status_bar_ && cargo test -p scriptcraft visible_page_ && cargo test -p scriptcraft toolbar_` — exit 101 at `status_bar_`; the new breakpoint test used `f32::EPSILON`, which rounded away at 720 and made its near-boundary assertion false. Replaced it with a representable `0.1` logical-point offset.
  - A subsequent `cargo fmt --all -- --check && ...` — exit 1 only because rustfmt required the changed assertion to be collapsed. Ran `cargo fmt --all` again.
  - Final `cargo fmt --all && cargo fmt --all -- --check && cargo test -p scriptcraft status_bar_ && cargo test -p scriptcraft visible_page_ && cargo test -p scriptcraft toolbar_` — exit 0; format check passed; status-bar 13/13, visible-page 1/1, toolbar 28/28.
  - `cargo check --workspace` initially exited 0 with a dead-code warning on `StatusItemResponse::{response, activated}`; added a narrow documented allowance because the capabilities intentionally do not invent actions. Final `cargo check --workspace` exited 0 with no warnings.
  - `cargo test --workspace` — exit 0; app 91/91, core unit 45/45, integration suites 37/37, doc tests 0; no failures.
  - `cargo build --workspace` — exit 0.
  - `cargo clippy --workspace --all-targets -- -D warnings` — exit 0.
  - `git diff --check` — exit 0. Also checked the untracked new status module using `git diff --no-index --check /dev/null apps/scriptcraft/src/status_bar.rs`; no whitespace diagnostics (the normal no-index difference exit was normalized by the shell wrapper).
- **Final rerun after the wide three-region layout change:** `cargo fmt --all && cargo fmt --all -- --check && cargo test -p scriptcraft status_bar_ && cargo test -p scriptcraft visible_page_ && cargo test -p scriptcraft toolbar_` — exit 0 (13, 1, and 28 passed respectively); `cargo check --workspace` — exit 0, warning-free; `cargo test --workspace` — exit 0 (91 app, 45 core unit, 37 integration passed; 0 failures); `cargo build --workspace` — exit 0; `cargo clippy --workspace --all-targets -- -D warnings` — exit 0.
- **Acceptance evidence:** Focused selector/state/item/accessibility/currentness tests pass; the rendered scroll test selects a later viewport page after scroll with an unchanged caret; narrow fallback has deterministic breakpoint coverage; full workspace tests and required checks pass. Requirements `REQ-WRITING-LOOP-006` and the matching desktop-owned status/page blueprint contract are updated. Existing feature/technical/core ownership and core pagination/source mapping remain unchanged. No zoom controls, WO-003 work, core edits, or unrelated changes were introduced.
- **Native smoke:** Not run; this environment has no native-window/input automation. It is recommended, not a configured hard-stop gate, and is not claimed as passed.
- **Changed paths:** `.forge/requirements/features/screenplay-writing-loop.md`; `.forge/blueprints/containers/scriptcraft-desktop.md`; this WO; `.forge/initiatives/2026-10-28-status-bar/execution-log.md`; `apps/scriptcraft/src/main.rs`; `apps/scriptcraft/src/status_bar.rs` (new). Pre-existing WO-001 completion/validation records remain preserved and unchanged by this continuation.
- **Git/operations boundary:** Branch remains `dev/2026-10-28-status-bar`, HEAD remains `fde481db08ba058a1327e2d23abb9b056c2556c3`; index remains empty. No staging, commit, reset, stash, merge, or branch operation occurred. WO-003 remains unstarted.
- **Validation boundary:** This is an implementer handoff only. No independent validation or completion is claimed; a separate fresh validator must compare the changes and assess the applicable gates.
- **Result/next action:** WO-002 is `in-review` with implementation evidence and both actual implementer host dispatch IDs reconciled. A separate fresh validator must handle WO-002; keep WO-003 undispatched until its dependency handoff.

## Summary

Create the desktop-owned item-based status bar and viewport-page presentation. Support noninteractive text, clickable text and clickable icons as capabilities without inventing actions/icons; provide truthful center clean/unsaved, cumulative frames, path, operation/error status and safe diagnostic access with a deterministic narrow-width fallback. The left page item reports the currently visible rendered sheet from actual page geometry and the editor's actual scroll clip—not caret position—and never presents stale page information as current.

## In scope

- Consume page rectangles and clip/scroll geometry produced by the editor's current `ScreenplayLayout` rendering path in [`main.rs`](../../../../apps/scriptcraft/src/main.rs); do not estimate pages from caret position, page index under cursor, nominal viewport center, or fixed pixel thresholds.
- Add a discrete desktop-owned item-based status-bar model/renderer (for example, `status_bar.rs`) with noninteractive text, clickable text and clickable-icon item capabilities. Capabilities must not invent or instantiate an action/icon not requested by the user.
- Compose left, center and right regions. The left region contains the viewport-page item; the center contains current clean/unsaved state, cumulative frames, path and operation/error status with access to privacy-safe diagnostics. Leave right-region zoom behavior to WO-003, using a neutral/available slot without implementing zoom controls here.
- Read live `EditingSession::dirty()` and represent currently clean versus unsaved truthfully; a clean in-memory project does not imply it was previously saved. Preserve current error visibility, path and operation status, and access to bounded native-input/performance diagnostics, using compact disclosure where needed. Do not expose document/source text, serialized project data or sensitive diagnostic payloads.
- Provide deterministic bounded narrow-width fallback that retains access to page, clean/unsaved, frames, path, operation/error status and diagnostics. Interactive item capabilities have keyboard focus/activation and explicit accessible names; decorative/noninteractive text owns no competing response, focus or cursor.
- Define a testable desktop-owned page selector: for each rendered sheet, compute its vertical overlap with the actual editor scroll clip; choose the greatest overlap; exact overlap ties choose the smaller/earlier rendered page index.
- If the clip is in an inter-page gap and no page has positive overlap, select the nearest rendered page by vertical interval distance; exact distance ties choose the earlier page.
- Use one-based visible numbering and total count over **all rendered sheets**, including an optional title page plus body pages from `ScreenplayLayout.pages`. Keep title-page membership from `LayoutPage.title_page`; do not conflate title page with body caret/source-map indexing.
- Report an explicit unavailable state if the layout is unavailable/empty, rendered page geometry is invalid, clip geometry is invalid, or the geometry snapshot is stale. Never silently keep a previous `N of total` when it no longer describes current geometry.
- Keep page state current through passive vertical scroll, resize, presentation-scale/zoom changes, layout reflow, and conditional caret reveal. Account for the bottom status panel being laid out before the central editor: key/cache geometry by sufficient current layout revision, presentation scale/size, clip and scroll state; invalidate when inputs/events can make the last editor geometry stale, and request a repaint after fresh geometry is published. A bounded unavailable frame is preferable to a stale number.
- Ensure page status updates do not request caret reveal, move the viewport, steal editor focus, or change editor pointer/cursor/input ownership.
- Add item capability, state projection, error/path/diagnostic, accessibility/focus, narrow-fallback, page-selector and rendered-geometry regressions, including visible page differing from caret page and currentness across each named geometry change.

## Out of scope

- Toolbar extraction and preservation (WO-001).
- Zoom-control interaction and horizontal navigation implementation (WO-003); this WO must tolerate presentation-scale changes and keep its page result current.
- Changes to core layout/pagination, title-page content, source mappings, PDF/export geometry, page numbering inside the screenplay, caret reveal policy, editing/focus/input semantics, persisted data, or core crate APIs.
- Turning native smoke into a project hard-stop; none is configured in the current manifest.

## Requirements

Applicable existing requirements (verbatim):

- **REQ-WRITING-LOOP-002 / AC-WRITING-002.1:** “Given the same project and fixed page profile, the system shall derive stable page structure and source mappings independent of window zoom.”
- **REQ-WRITING-LOOP-002 / AC-WRITING-002.4:** “Given a document position and rendered geometry, the system shall support a safe caret/hit-test round trip at valid text boundaries.”
- **NFR-WRITING-001:** “Production paths should prefer typed errors and bounded inputs over panic-prone handling; a discovered crash or data-loss defect requires a regression test.”
- **NFR-WRITING-002:** “Core editing, layout, and persistence behavior shall be exercisable without a desktop window for deterministic tests.”

Initiative-confirmed behavior proposed at planning time for the additive requirement update: text, clickable-text and clickable-icon item capabilities without invented actions/icons; truthful current clean/unsaved state, cumulative frames, path and operation status; visible errors and access to privacy-safe diagnostics; deterministic bounded narrow-width fallback; and viewport-visible page selection (not caret-based), where greatest vertical overlap against the actual editor scroll clip wins, ties go earlier, gaps select nearest/ties earlier, total includes a rendered title page, and stale/failed geometry is explicitly unavailable. These requirements are now recorded as `REQ-WRITING-LOOP-006 / AC-WRITING-006.1–006.6` below and in the linked feature requirement.

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md) — desktop owns item-based status presentation and viewport rendering; update for status capabilities/data ownership, narrow fallback, page selection/currentness and status-before-editor invalidation.
- [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md) — shared fixed-layout and source-map boundary; verify unchanged.
- [`technical-context.md`](../../../blueprints/technical-context.md) — desktop/core separation and presentation-only scale; verify unchanged.
- [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md) — layout pages/title page/source mapping remain core-owned and unchanged.

## Implementation plan

1. Re-read the approved WO-001 toolbar boundary, current status/footer and actual editor `ScrollArea`; define the status item model/renderer and three-region composition without changing the editor response/ID or core layout API.
2. Project truthful center state, preserving error visibility, path, operation status, cumulative frames and bounded privacy-safe diagnostics. Implement keyboard/accessibility behavior and deterministic bounded narrow fallback that preserves access to critical status and page information; do not implement zoom controls.
3. Add a pure/testable page selector over ordered page rectangles and clip interval. Use finite valid rect checks, overlap length, interval distance, stable earlier-page tie breaks, and explicit unavailable result; use checked/defensive indexing rather than panic-prone assumptions.
4. Publish editor geometry/currentness to the status adapter without making page rectangles independent widgets. Since status is rendered before editor, validate cached geometry against the current frame's viewport/layout/presentation/scroll invalidation signals; if uncertain, render unavailable until fresh geometry arrives and request repaint after publication.
5. Add deterministic tests for item capabilities, state projection, accessibility/focus, narrow fallback, failures, overlap/ties, gaps/ties, first/last bounds, title-plus-body numbering, off-screen caret, geometry failure and invalidation after scroll/resize/zoom/reflow/caret reveal. Verify page selection is observational only and does not mutate/recenter editor state.
6. Update only the authorized requirements and desktop-blueprint wording below; run focused and configured project checks and preserve comparable changed-file evidence.

## Files and systems

- **Create:** A discrete status-bar item/model/renderer module and pure desktop page-selector/currentness helpers or bounded submodule; tests for item behavior, status projection, selector/geometry and rendered status integration.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs) and the status-bar module for status composition and geometry publication/rendering; [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md); [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md).
- **Avoid changing:** `crates/scriptcraft-core/`, `ScreenplayLayout` page generation/source mapping, title-page serialization, editor input/focus/caret-reveal semantics, PDF, persistence/schema, and toolbar behavior.

## Validation and E2E acceptance tests

- **Automated checks:** [`cargo fmt --all -- --check`](../../../../Cargo.toml:1); `cargo test -p scriptcraft status_bar_`; `cargo test -p scriptcraft visible_page_`; `cargo test -p scriptcraft toolbar_`; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`. These are future commands, not results from this planning session.
- **Focused status-item matrix:** Noninteractive text, clickable text and clickable icon remain capability types only, with no invented actions/icons; clean in-memory state is distinct from unsaved, save/failed save/open project truthful state/path/status, cumulative frames use the existing counter, errors remain visible, safe diagnostics remain reachable and contain no document text; keyboard activation/accessibility labels work for interactive items and decorative text remains noninteractive; narrow fallback preserves access to page and status/error/path/diagnostics.
- **Focused selector matrix:** Positive overlap selects maximum height; exact overlap tie chooses earliest page; pages with unequal overlap choose the greatest even if caret is elsewhere; clip in gap chooses nearest; equal gap distance chooses earlier page; clip above/below all sheets chooses nearest edge page; invalid/empty/no-layout geometry returns explicit unavailable; output is one-based with exact rendered-page count including optional title page. Test title-page sheet and first body sheet separately.
- **Rendered/currentness matrix:** Scroll to another page while caret stays elsewhere; put caret far offscreen and verify no caret-follow; then perform caret reveal and verify current page only after geometry is fresh; trigger resize, scale/zoom, reflow/revision and actual scroll input; on any mismatch show unavailable rather than previous page, then refresh to correct value; verify no page status event scrolls, focuses or edits the editor. Retain title/body page distinction and failure path.
- **Manual/external checks:** Recommended bounded desktop smoke at normal and narrow widths with a multi-page project and optional title sheet. Observe all three item capabilities as used without invented actions, center clean/unsaved/frame/path/operation/error and accessible safe diagnostics, narrow fallback, and page selection with caret on a different/off-screen page while scrolling across page boundaries/gaps, resizing and triggering caret reveal. Also observe zoom-change invalidation/currentness without implementing zoom controls. If window/input unavailable, record unavailable honestly; this is not a configured project hard-stop.
- **Acceptance test:** Given a clean/unsaved session or failed operation, when the item-based status renders, then it accurately represents current clean/unsaved state and cumulative frames, preserves path and operation status, visibly retains errors and safe diagnostic access, exposes only the three interaction capabilities without invented actions/icons, and remains keyboard-accessible/reachable at narrow widths. Given rendered pages and an actual editor clip, when one or more pages overlap, then status displays the one-based page with greatest vertical overlap and ties earlier; given a gap, then nearest-page/ties-earlier applies; given title page, it counts in total; given caret on a different page, the result follows viewport only; and given stale/failed geometry after any relevant viewport/layout change, status reports unavailable until current geometry is published rather than displaying stale `N of total`.
- **Independent validator boundary:** A separate fresh-context validator after implementation assesses only this WO, approved plan/review, source/test/diff evidence, durable updates and independently rerun commands. It must distinguish deterministic/rendered geometry evidence from actual native-window smoke; unavailable native observation is never a claimed pass.

## Durable-document impact

- **Requirements impact:** `update` — selected: add additive user-visible criteria for item capabilities, truthful center clean/unsaved and cumulative frame state, path/operation/error/diagnostic access, narrow fallback, and viewport-derived visible-page reporting with deterministic overlap/gap/title/unavailable behavior to [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md); preserve existing toolbar, fixed-page and source-mapping criteria.
- **Blueprint impact:** `update` — selected: record desktop-owned item-based status capabilities/data ownership, privacy-safe center presentation, narrow fallback, and page geometry publication/currentness, tie/gap behavior and status-before-editor stale-snapshot boundary in [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md).
- **Affected documents/IDs:** Preserved `REQ-WRITING-LOOP-002 / AC-WRITING-002.1, AC-WRITING-002.4` and `REQ-WRITING-LOOP-005 / AC-WRITING-005.1–005.5, AC-WRITING-005.7`; added `REQ-WRITING-LOOP-006 / AC-WRITING-006.1–006.6` in `screenplay-writing-loop.md`; added the desktop status/page contract in `scriptcraft-desktop.md`.
- **Rationale and evidence:** Existing requirements guarantee toolbar behavior, zoom-independent fixed page structure and valid geometry mapping, but do not define item-based status or viewport-visible page reporting. Current `live_document_surface()` has actual page rectangles and actual scroll clipping, while bottom status renders first; desktop-specific currentness is needed without changing the core layout contract. Feature, technical and core blueprints remain unchanged if implementation stays app-owned.
- **Authorized documentation changes:** The authorized additive status/page criteria and matching desktop-owned item/presentation/geometry-currentness contract are applied. Existing toolbar, fixed-page and source-mapping requirements are preserved; feature, technical-context and core blueprints are verified unchanged.

## Completion checklist

- [x] Implementation is complete within scope.
- [x] Tests/checks and required gates have evidence; the optional native smoke limitation is recorded.
- [x] Durable-document impact classification is accurate against the diff, and authorized documentation updates are applied.
- [x] Execution log is appended with implementation evidence and both actual implementer host dispatch IDs. Independent validation is a separate next-phase artifact and is not claimed here.
- [x] Work order is `in-review` with evidence; completion comes only from passed validation.

# Validation: WO-002a — Freshness and Response-Free Status Text

## Validation metadata

- **Review type:** `validation`
- **Status:** `complete` — fresh independent validation is complete with outcome `pass`; actual host dispatch identity and resulting work-order transitions are reconciled below.
- **Validator:** Fresh independent LIGHT validator, distinct from the WO-002 implementers, prior failed validator, and WO-002a implementer.
- **Validator host dispatch ID:** `01a183d2-d3db-75f9-b653-b8c72106d86b` (actual host dispatch ID, reconciled by the orchestrator). The host does not expose a stable internal session/agent ID; none is inferred.
- **Session exception:** `none`
- **Date:** 2026-10-28
- **Initiative:** [`status-bar-v1.md`](../status-bar-v1.md)
- **Validation file:** `validation-WO-002a.md`
- **Validated work order:** [`WO-002a-freshness-and-noninteractive-text.md`](../work-orders/WO-002a-freshness-and-noninteractive-text.md)
- **Compared fix range:** `4be598803fe19c8f212115ae0f2455afef412fac..a73c9de14195fb32ae36dce3ae2bba090c8970cf`; target is a direct child of base.
- **Full original WO-002 comparison:** `fde481db08ba058a1327e2d23abb9b056c2556c3..a73c9de14195fb32ae36dce3ae2bba090c8970cf`; target is a descendant of base and includes both the original WO-002 commit and its fix.
- **Fix chain:** Original [`WO-002-status-items-and-viewport-page.md`](../work-orders/WO-002-status-items-and-viewport-page.md), preserved failed [`validation-WO-002.md`](validation-WO-002.md), and this narrow fix validation.

## Authorized scope and comparison basis

The approved scope is the two hard-stop findings in WO-002's prior independent validation: invalidate cached rendered-page geometry on each successful New/Open session replacement before status projection, including revision-zero aliasing; and draw noninteractive status text without creating an egui response while retaining visible/readable and accessible text. This review also explicitly re-assesses every original WO-002 acceptance criterion against the combined result. No WO-003 zoom work or unrelated implementation is in scope.

The approved initiative plan and passing design review authorized a desktop-owned item-based status bar, truthful center status and privacy-safe diagnostics, narrow fallback, and viewport-derived page selection/currentness, while deferring zoom controls to WO-003. The governing artifacts are [`status-bar-v1.md`](../status-bar-v1.md), [`design-review.md`](../reviews/design-review.md), [`WO-002-status-items-and-viewport-page.md`](../work-orders/WO-002-status-items-and-viewport-page.md), [`WO-002a-freshness-and-noninteractive-text.md`](../work-orders/WO-002a-freshness-and-noninteractive-text.md), the original failed [`validation-WO-002.md`](validation-WO-002.md), and the manifest policy in [`project-manifest.md`](../../../project-manifest.md).

Live repository evidence collected for this validation:

- Branch is `dev/2026-10-28-status-bar`; live `HEAD` is `a73c9de14195fb32ae36dce3ae2bba090c8970cf`.
- The fix commit's direct parent is `4be598803fe19c8f212115ae0f2455afef412fac`; the original WO-002 commit's direct parent is `fde481db08ba058a1327e2d23abb9b056c2556c3`.
- The fix-only range modifies exactly [`main.rs`](../../../../apps/scriptcraft/src/main.rs), [`status_bar.rs`](../../../../apps/scriptcraft/src/status_bar.rs), and [`toolbar.rs`](../../../../apps/scriptcraft/src/toolbar.rs) (349 insertions, 10 deletions). The full requested `fde..a73` range contains nine paths: [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md), [`execution-log.md`](../execution-log.md), [`validation-WO-001.md`](validation-WO-001.md), [`WO-001-toolbar-extraction.md`](../work-orders/WO-001-toolbar-extraction.md), [`WO-002-status-items-and-viewport-page.md`](../work-orders/WO-002-status-items-and-viewport-page.md), [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md), [`main.rs`](../../../../apps/scriptcraft/src/main.rs), [`status_bar.rs`](../../../../apps/scriptcraft/src/status_bar.rs), and [`toolbar.rs`](../../../../apps/scriptcraft/src/toolbar.rs).
- The three fix-range source paths are directly authorized by WO-002a. The original range's additive requirement/desktop-blueprint changes were authorized by WO-002. Its WO-001 completion/validation records are protected same-initiative evidence documented in the original WO and failed validation, not unrelated product behavior. The [`toolbar.rs`](../../../../apps/scriptcraft/src/toolbar.rs) fix-range change is the successful-New invalidation required by finding A. No core, persistence, schema, export, manifest, dependency, or WO-003 change is present. No clearly unrelated or rogue behavior was found.
- The Git index is empty. The only live worktree changes are Forge evidence records: modified initiative log, plan, original WO-002 record and WO-002a record, plus untracked original failed validation, fix WO and this new validation artifact; application source is clean relative to `HEAD`. No stage, commit, branch, reset, stash, or merge operation was performed by this validator. The pre-existing failed validation was read and preserved, not overwritten.

## Acceptance-by-acceptance findings

| Acceptance boundary | Result | Independent evidence |
|---|---|---|
| **WO-002a finding A / AC-WRITING-006.6 — successful New with revision alias** | **PASS** | The real toolbar action dispatcher replaces the session, then invalidates page geometry in the same successful branch before returning; the app's update path renders the toolbar before querying page status. See [`dispatch_toolbar_action()`](../../../../apps/scriptcraft/src/toolbar.rs:469), [`update()`](../../../../apps/scriptcraft/src/main.rs:1101), and the invalidation in [`toolbar.rs`](../../../../apps/scriptcraft/src/toolbar.rs:479). The regression [`visible_page_new_replacement_invalidates_revision_alias_before_projection()`](../../../../apps/scriptcraft/src/main.rs:1367) seeds old 3-page geometry at revision 0, dispatches New to a revision-0 replacement with unchanged viewport/scale, observes `Unavailable` on the first projection, then renders fresh editor geometry and observes page 1 of 1, repaint requested, selection unchanged, and no surface focus. This tests the production dispatcher used by the toolbar path, not only a helper. |
| **WO-002a finding A / AC-WRITING-006.6 — successful Open, failure preservation** | **PASS** | Successful [`open_native_at()`](../../../../apps/scriptcraft/src/main.rs:166) replacement invalidates immediately after `replace_with_saved()` succeeds and before the next `update()` status query ([`main.rs`](../../../../apps/scriptcraft/src/main.rs:181)). [`visible_page_open_replacement_invalidates_revision_alias_and_failed_open_does_not()`](../../../../apps/scriptcraft/src/main.rs:1413) seeds an old revision-0 snapshot with total 1, opens a saved multi-page project whose session also has revision 0 at the same viewport/scale, observes first-projection `Unavailable`, then fresh rendered page 1 with the new total and a repaint request. Selection/focus remain unchanged. Its failed-Open control retains the existing path/error semantics and old published snapshot. |
| **WO-002a finding B / AC-WRITING-006.1 — response-free noninteractive text** | **PASS** | [`render_item()`](../../../../apps/scriptcraft/src/status_bar.rs:100) routes text to [`paint_noninteractive_text()`](../../../../apps/scriptcraft/src/status_bar.rs:130), which lays out/paints via painter and cursor allocation/advancement, makes no `Response` or `interact` call, and supplies an AccessKit Label with text and bounds. The actual-render regression [`status_bar_item_rendering_keeps_text_noninteractive_and_buttons_named()`](../../../../apps/scriptcraft/src/status_bar.rs:732) confirms emitted readable text shape and AccessKit label, then moves and clicks over its bounds and checks the egui interaction snapshot has no pointer/hover/click/drag target at the would-be label ID, no focus, and default cursor. This is not merely an adapter `None` assertion. Existing clickable text/icon and explicit accessible names remain covered in the same test; Enter/Space activation remains covered by the focused status tests. |
| **AC-WRITING-006.1 — interaction capabilities and accessible labels** | **PASS** | The item model keeps plain text, clickable text, and clickable-icon as distinct capabilities without storing/inventing actions or icons ([`StatusItemKind`](../../../../apps/scriptcraft/src/status_bar.rs:11), [`render_item()`](../../../../apps/scriptcraft/src/status_bar.rs:100)). Plain text is response-free as above. Clickable controls retain Button semantics, accessible names and keyboard activation. The model, rendering/accessibility and focused activation tests pass. |
| **AC-WRITING-006.2 — truthful live document and operation state** | **PASS** | [`project_center()`](../../../../apps/scriptcraft/src/status_bar.rs:382) distinguishes dirty, clean saved and clean not-saved state, projects cumulative frames, current path, operation and error. Projection regression checks clean-unsaved, dirty/save-error/path and clean-saved cases. Actual app update supplies live session and frame/path/status/error fields. |
| **AC-WRITING-006.3 — visible failures and privacy-safe diagnostics** | **PASS** | The status renderer paints an error visibly in red and keeps diagnostics reachable through a collapsed disclosure ([`show()`](../../../../apps/scriptcraft/src/status_bar.rs:448)); [`safe_summary()`](../../../../apps/scriptcraft/src/status_bar.rs:423) limits output to event categories and counters. The diagnostic test checks required aggregate fields and absence of document/source-text markers. No document text or serialized project content is projected. |
| **AC-WRITING-006.4 — deterministic narrow fallback** | **PASS** | The compact breakpoint is deterministic and tested immediately below/at the threshold plus non-finite width. The compact layout keeps page/clean/frames on its first wrapped row and path/operation/error on its second, while diagnostics remain reachable ([`show()`](../../../../apps/scriptcraft/src/status_bar.rs:453)). Status-focused regressions pass. |
| **AC-WRITING-006.5 — rendered page selection, overlaps/ties/gaps/bounds/title total** | **PASS** | [`select_visible_page()`](../../../../apps/scriptcraft/src/status_bar.rs:234) validates finite positive rectangles; uses greatest vertical overlap with stable earlier-index tie resolution, then nearest interval and stable earlier tie for a gap; reports one-based numbering and total rendered rectangles. Selector tests cover overlap and equal ties, gap and equal ties, above/below bounds, optional title plus body count, and empty/invalid geometry. The editor uses `ScreenplayLayout.pages` rendered rectangles and actual `ScrollArea` clip intersection in [`live_document_surface()`](../../../../apps/scriptcraft/src/main.rs:481) and publishes actual clip/page geometry at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:556), not caret-derived estimates. |
| **AC-WRITING-006.6 — unavailable on stale/failure; currentness after inputs change** | **PASS** | [`ViewportPageTracker::visible_page()`](../../../../apps/scriptcraft/src/status_bar.rs:303) returns unavailable for invalidation, absent snapshot, non-adjacent frame, revision/viewport/scale mismatch, or empty geometry, and requests repaint after fresh publication; [`invalidate()`](../../../../apps/scriptcraft/src/status_bar.rs:363) clears the snapshot. Both successful replacement regressions cover session-revision aliasing now. Existing tracker/input tests and rendered scroll/caret-reveal tests cover stale frames and current geometry. The editor publishes after rendering, requests repaint when changed, and updates status before the editor on the next frame. |
| **Original WO-002 status layout and side-effect boundary** | **PASS** | Wide layout has left page, center state and neutral reserved right region (no WO-003 controls); compact fallback preserves required status. Page tracking consumes actual geometry and does not request caret reveal, scroll, focus, or edit. Rendered passive scroll test changes visible page while caret remains offscreen/unchanged; New/Open tests assert selection/focus unchanged. Failed Open preserves prior projection and user-visible error/path/session behavior. |
| **Original toolbar regression/preservation boundary (WO-001 dependency)** | **PASS** | Fresh `cargo test -p scriptcraft toolbar_` passed all 28 tests, including order, allocation, narrow overflow, routing, hover/cursor/focus and geometry cases. The fix's one-line `toolbar.rs` change only invalidates viewport-page state on successful New and does not alter toolbar layout/interaction. |
| **Existing fixed-layout/source-map/core requirements — AC-WRITING-002.1, AC-WRITING-002.4, NFR-WRITING-001/002** | **PASS** | No core crate, layout/pagination, source mapping, persistence/export or API files changed in either requested range. Status computes presentation from rendered geometry only. Workspace tests/check/build and strict Clippy all pass; no conflict with fixed-profile layout or valid geometry/editing contracts was found. |

## Findings

### Blockers

- None. The two prior hard-stop findings are fixed in the actual application paths and directly covered by regressions. The full original WO-002 acceptance boundary passes against the combined implementation.

### Warnings

- **Native-window visual/input smoke unavailable — Severity: warning.** The bounded `cargo run -p scriptcraft` process exited within 15 seconds with exit code 0. This is process launch/termination evidence only: no native screenshot, pointer/keyboard interaction, live visual/layout inspection, or interactive smoke is claimed. The manifest and WO classify native smoke as recommended and not a configured hard-stop gate. Deterministic egui rendering tests and all required automated acceptance/gates were run.

### Notes

- No deceptive, backdoor, malicious, nefarious, clearly unrelated, or rogue behavior was found.
- WO-002a's durable-document classification is `verify unchanged`; no requirement or blueprint change appears in the fix range. The existing `REQ-WRITING-LOOP-006.1/.6` and desktop-owned response-free/currentness contracts are restored, not expanded.
- Prior failed validation [`validation-WO-002.md`](validation-WO-002.md), its result/findings, and implementation-time failed attempts remain preserved unchanged. This artifact does not rewrite the prior outcome.
- No visual screen capture/native interaction was available. The output of `cargo run` is explicitly not treated as evidence of a visible window.

## Required command results — fresh validator reruns

Every required command was run at live `HEAD` and exited 0. Focused counts below are fresh independent results, not implementer claims.

| Command | Fresh result |
|---|---|
| `cargo fmt --all -- --check` | Exit 0; passed. |
| `cargo test -p scriptcraft status_bar_` | Exit 0; 13 passed, 0 failed, 80 filtered out. |
| `cargo test -p scriptcraft visible_page_` | Exit 0; 3 passed, 0 failed, 90 filtered out. |
| `cargo test -p scriptcraft toolbar_` | Exit 0; 28 passed, 0 failed, 65 filtered out. |
| `cargo test -p scriptcraft status_bar_item_rendering_keeps_text_noninteractive_and_buttons_named` | Exit 0; 1 passed, 0 failed, 92 filtered out. |
| `cargo check --workspace` | Exit 0; passed, no warnings. |
| `cargo test --workspace` | Exit 0; 93 app tests + 45 core unit tests + 37 core integration tests = 175 passed, 0 failed; 0 doc tests. |
| `cargo build --workspace` | Exit 0; passed. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Exit 0; passed. |
| `cargo run -p scriptcraft` (Python-bounded 15-second process window) | Cargo launch and application process exited within 15 seconds, exit code 0. Startup/termination only; no visual/input smoke. |
| `git diff --check fde481db08ba058a1327e2d23abb9b056c2556c3..a73c9de14195fb32ae36dce3ae2bba090c8970cf` | Exit 0; full original-plus-fix committed range clean. |
| `git diff --check 4be598803fe19c8f212115ae0f2455afef412fac..a73c9de14195fb32ae36dce3ae2bba090c8970cf` | Exit 0; fix committed range clean. |
| `git diff --check` | Exit 0; current tracked worktree evidence clean. |

There were no failed commands during this fresh validation rerun. All required project gates and explicit automated acceptance checks pass. Historical failures/repairs recorded by implementers and the previous failed validator are not erased or reclassified.

The post-recordkeeping `git diff --check` exited 0 for tracked worktree changes. `git diff --no-index --check /dev/null` against the new validation artifact, the untracked WO-002a work-order record, and the preserved original failed validation each emitted no whitespace diagnostics; each returned exit 1 solely because the no-index files differ from `/dev/null`.

## Decision

- **Outcome:** `pass`
- **Required next state:** Orchestrator reconciliation may set WO-002a and original WO-002 to `complete`; this validator did not change either work-order status, initiative phase/status, or any other workflow state.
- **Fix work order required:** `no`
- **Validation scope result:** WO-002a's two findings pass and the full original WO-002 acceptance boundary passes on the combined comparison target.
- **State boundary:** WO-002a remains `in-review` and original WO-002 remains `blocked` in the current records pending the orchestrator's separate reconciliation. WO-003 remains `draft` and undispatched. No manual approval is requested or recorded.

## Orchestrator result reconciliation — 2026-10-28

- **Actual validator host dispatch ID:** `01a183d2-d3db-75f9-b653-b8c72106d86b`. No internal session/agent ID was exposed or inferred.
- **Confirmed pass coverage:** The PASS applies both to WO-002a `4be598803fe19c8f212115ae0f2455afef412fac..a73c9de14195fb32ae36dce3ae2bba090c8970cf` and full WO-002 `fde481db08ba058a1327e2d23abb9b056c2556c3..a73c9de14195fb32ae36dce3ae2bba090c8970cf`. The artifact documents resolution of both prior blockers, full original acceptance coverage, and passing mandatory focused/workspace/project gates. No blocker remains.
- **Transition:** Following the pass, WO-002a transitions `in-review` → `complete`, and original WO-002 transitions `blocked` → `complete`. Initiative remains `execute` / `active`; WO-003 remains `draft` and undispatched.
- **Limitations/history:** Native desktop visual/input smoke remains unavailable and is not claimed as passed; it is optional and not a configured hard-stop gate. The historical failed [`validation-WO-002.md`](validation-WO-002.md) remains unchanged and preserved.

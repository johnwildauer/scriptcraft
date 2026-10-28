# Validation: WO-002 — Status Items and Viewport Page

## Validation metadata

- **Review type:** `validation`
- **Status:** `complete` — independent implementation-alignment validation is complete with outcome `fail`; the orchestrator's host-ID/state/fix-order reconciliation is recorded below.
- **Validator:** Fresh independent LIGHT validator, separate from the initial implementer and continuation implementer.
- **Validator session/agent ID:** Host does not expose a stable internal session/agent ID; none is inferred.
- **Host dispatch ID:** `01a183a1-84ed-71a8-bc37-8608a4b1633a` (actual fresh validator host dispatch ID, reconciled by the orchestrator; no internal session/agent ID is inferred).
- **Session exception:** `none`
- **Date:** 2026-10-28
- **Initiative:** [`status-bar-v1.md`](../status-bar-v1.md)
- **Validation file:** `validation-WO-002.md`
- **Validated work order:** [`WO-002-status-items-and-viewport-page.md`](../work-orders/WO-002-status-items-and-viewport-page.md)
- **Compared commit/diff:** `fde481db08ba058a1327e2d23abb9b056c2556c3..4be598803fe19c8f212115ae0f2455afef412fac`; target is direct child of base.
- **Fix chain:** At validator completion, none existed. The validator recommended `WO-002a-freshness-and-noninteractive-text.md`; the orchestrator later created it as `draft` (not dispatched), as recorded below. This validator did not create it or change WO status.

## Authorized scope and comparison basis

The approved boundary is the canonical WO-002 and its aligned plan/review: add the desktop-owned status Item capability model and three-region presentation; preserve clean/unsaved, cumulative frame, path, operation, error, and privacy-safe diagnostic access; provide compact fallback; and report viewport-visible rendered page from actual editor page rectangles and actual scroll clip with deterministic overlap/gap/tie/title/unavailable rules and currentness after geometry changes. WO-003 zoom controls are explicitly deferred. The governing references are [`status-bar-v1.md`](../status-bar-v1.md), [`design-review.md`](../reviews/design-review.md), [`WO-002-status-items-and-viewport-page.md`](../work-orders/WO-002-status-items-and-viewport-page.md), [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md), and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md).

Live Git evidence collected before writing this artifact:

- Branch: `dev/2026-10-28-status-bar`.
- `HEAD` is `4be598803fe19c8f212115ae0f2455afef412fac`; its direct parent is `fde481db08ba058a1327e2d23abb9b056c2556c3`.
- The index was empty. Before this validation's documentation edits, the only worktree changes were unstaged recordkeeping additions to [`execution-log.md`](../execution-log.md) and [`WO-002-status-items-and-viewport-page.md`](../work-orders/WO-002-status-items-and-viewport-page.md), both documenting the commit hash/base. There were no untracked paths.
- The committed range changed exactly eight paths: [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md), [`execution-log.md`](../execution-log.md), [`validation-WO-001.md`](validation-WO-001.md), [`WO-001-toolbar-extraction.md`](../work-orders/WO-001-toolbar-extraction.md), [`WO-002-status-items-and-viewport-page.md`](../work-orders/WO-002-status-items-and-viewport-page.md), [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md), [`main.rs`](../../../../apps/scriptcraft/src/main.rs), and [`status_bar.rs`](../../../../apps/scriptcraft/src/status_bar.rs). The two app source paths and the additive requirement/desktop-blueprint edits are authorized. WO-001 completion/validation records are same-initiative protected evidence explicitly inventoried in the WO/log; no other source path changed.
- No core crate, Cargo manifest/lockfile, schema, persistence, export, toolbar module, or WO-003 implementation changed. The compared source diff is limited to the status module and integration in `main.rs`.
- The post-commit worktree documentation does not alter implementation. No staging, commit, reset, stash, merge, branch switch, WO-status change, or WO-003 dispatch was performed by this validator.

The independent comparison is meaningful: I inspected the committed source implementation, test code, authorized requirement/blueprint updates, changed-path inventory, canonical WO evidence, and prior WO-001 validation/completion record. Implementer command history, including the initial failure and subsequent repair, was treated as reported evidence; all required project commands were independently rerun below.

## Acceptance-by-acceptance findings

| Criterion | Result | Evidence |
|---|---|---|
| **AC-WRITING-006.1 — Item capabilities, accessibility, and noninteractive text** | **FAIL (blocker; see Finding B)** | [`StatusItemKind`](../../../../apps/scriptcraft/src/status_bar.rs:11) separates text/clickable-text/clickable-icon capabilities; interactive rendering labels controls with [`interactive_response()`](../../../../apps/scriptcraft/src/status_bar.rs:73) and recognizes Enter/Space activation. However, text rendering calls `ui.label()` at [`render_item()`](../../../../apps/scriptcraft/src/status_bar.rs:98), whose egui 0.29.1 `Label` uses default `Sense::hover()` and creates a pointer-hover `Response`; returning `None` from this adapter does not remove that underlying response. The accepted contract explicitly says noninteractive text owns no pointer response, and the desktop blueprint says response-free. The current test at [`status_bar_item_rendering_keeps_text_noninteractive_and_buttons_named()`](../../../../apps/scriptcraft/src/status_bar.rs:643) checks only the adapter's returned `Option`, not the label's actual `Sense`. No unrequested action/icon is installed. |
| **AC-WRITING-006.2 — Truthful live document/operation state** | PASS | [`project_center()`](../../../../apps/scriptcraft/src/status_bar.rs:299) distinguishes dirty, clean named, and clean unnamed state; projects cumulative frames, native path, operation, and error. The session values are supplied in [`update()`](../../../../apps/scriptcraft/src/main.rs:1117). Projection tests cover clean-unsaved, dirty/path/save error, and clean saved state at [`status_bar_center_projection_reports_clean_unsaved_path_and_errors()`](../../../../apps/scriptcraft/src/status_bar.rs:597). |
| **AC-WRITING-006.3 — Visible errors and privacy-safe diagnostics** | PASS | [`show()`](../../../../apps/scriptcraft/src/status_bar.rs:364) visibly renders errors and exposes a collapsed diagnostics disclosure. [`safe_summary()`](../../../../apps/scriptcraft/src/status_bar.rs:340) formats only focus/event categories and aggregate counters; the privacy test at [`status_bar_diagnostic_projection_contains_only_categories_and_counters()`](../../../../apps/scriptcraft/src/status_bar.rs:621) checks category/counter presence and excludes document/source-text markers. No screenplay or serialized project content is passed to the diagnostics projection. |
| **AC-WRITING-006.4 — Deterministic compact layout** | PASS | [`uses_compact_status_layout()`](../../../../apps/scriptcraft/src/status_bar.rs:6) selects compact mode below 720 logical points and for non-finite width. [`show()`](../../../../apps/scriptcraft/src/status_bar.rs:377) wraps visible page/clean/frames in the first row and path/operation/error in a second, with diagnostics disclosure retained. [`status_bar_uses_deterministic_compact_layout_below_breakpoint()`](../../../../apps/scriptcraft/src/status_bar.rs:588) covers both sides and a non-finite width. |
| **AC-WRITING-006.5 — Actual clip selection, ties, gaps, numbering, title page** | PASS | [`select_visible_page()`](../../../../apps/scriptcraft/src/status_bar.rs:151) validates clip/page geometry, chooses maximum vertical overlap using strict `>` (stable earlier-page tie), chooses nearest interval if overlap is zero (stable earlier tie), and returns one-based index with total rendered rectangles. The selector tests at [`status_bar.rs`](../../../../apps/scriptcraft/src/status_bar.rs:430) cover overlap/tie, gap/tie, above/below bounds, title-plus-body count, and invalid/empty geometry. The editor supplies rendered page rectangles and `scroll_output.inner_rect ∩ ui.clip_rect()` at [`live_document_surface()`](../../../../apps/scriptcraft/src/main.rs:480), not caret-derived estimates. |
| **AC-WRITING-006.6 — Independence/currentness/unavailable state** | **FAIL (blocker; see Finding A)** | Normal scroll invalidation, revision/viewport/scale mismatch, caret reveal invalidation, fresh publication and repaint are implemented by [`ViewportPageTracker::visible_page()`](../../../../apps/scriptcraft/src/status_bar.rs:220), [`publish()`](../../../../apps/scriptcraft/src/status_bar.rs:250), and [`page_geometry_input_invalidated()`](../../../../apps/scriptcraft/src/main.rs:1158). The rendered scroll test [`visible_page_tracks_actual_scrolled_geometry_without_following_offscreen_caret()`](../../../../apps/scriptcraft/src/main.rs:1234) uses actual editor rendering/scroll geometry and confirms caret unchanged. But successful New/Open session replacement is not a freshness key/invalidation: New assigns a new revision-0 session in [`dispatch_toolbar_action()`](../../../../apps/scriptcraft/src/toolbar.rs:469); Open calls [`replace_with_saved()`](../../../../apps/scriptcraft/src/main.rs:181), which replaces the session with a newly constructed revision-0 session ([`EditingSession::new()`](../../../../crates/scriptcraft-core/src/editing.rs:73), [`replace_with_saved()`](../../../../crates/scriptcraft-core/src/editing.rs:124)). [`update()`](../../../../apps/scriptcraft/src/main.rs:1103) runs the toolbar before querying visible-page state at lines 1109–1115. If the old snapshot and replacement session both have revision 0, viewport/scale are unchanged, and the toolbar click is outside the prior editor clip, [`visible_page()`](../../../../apps/scriptcraft/src/status_bar.rs:237) accepts the prior frame's snapshot and returns its old page/total for that status render before the editor publishes replacement geometry. That is stale `N of total` after a session/layout replacement, contrary to the required unavailable-until-fresh behavior. |
| **WO-002 editor ownership/currentness details (scroll, resize, scale, reflow, reveal; no focus/scroll side effects)** | PARTIAL; blocker A applies | Actual editor `ScrollArea` geometry is published in [`live_document_surface()`](../../../../apps/scriptcraft/src/main.rs:505) and status logic itself does not call reveal, scroll, edit, focus, or editor interaction. Frame age, revision, viewport, and scale checks are tested. The session identity alias described above is not covered and breaks the broad currentness contract for New/Open replacement. Resize/scale/revision behavior is covered by tracker input-mismatch tests at [`status_bar_tracker_requires_fresh_frame_and_matching_geometry_inputs()`](../../../../apps/scriptcraft/src/status_bar.rs:521); no actual native-window observation was available.
| **Existing fixed-layout/source-map/core/editor contracts (AC-WRITING-002.1, AC-WRITING-002.4; NFR-WRITING-001/002)** | PASS; no core change | `git diff` shows no core/API/layout/export/persistence change. The status uses scaled rendered rectangles only and leaves the existing editor's single whole-surface response in place at [`live_document_surface()`](../../../../apps/scriptcraft/src/main.rs:574). Workspace tests/check/build/strict Clippy pass below. No contradiction to fixed pagination or source-map contracts was found in the authorized diff. |

## Findings

### Blockers

- **A — Session replacement can publish a stale page indicator — Severity: blocker.** Evidence: New replaces the complete `EditingSession` with `EditingSession::new()` (revision resets to 0) at [`toolbar.rs`](../../../../apps/scriptcraft/src/toolbar.rs:471); successful Open uses `replace_with_saved()` at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:181), which constructs a fresh revision-0 session at [`editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs:124). The top toolbar is rendered before the page status query at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1103). The cache checks only adjacent frame, session revision, viewport, scale, and explicit input invalidation in [`status_bar.rs`](../../../../apps/scriptcraft/src/status_bar.rs:237); no New/Open success path invalidates it or uses a session-generation key. Therefore an old snapshot can satisfy every predicate after a same-revision replacement and be displayed once before the central editor publishes the new layout. This plainly fails AC-WRITING-006.6 and the WO's no-stale-page rule. **Recommendation / precise fix scope:** invalidate the page snapshot on each successful New/Open session replacement before the status projection runs, or include a monotonically changing session/layout generation in both snapshot publication and consumption. Ensure the frame remains `Page unavailable` until the replacement editor publishes fresh geometry and requests repaint. Add a regression that replaces a revision-0 session with another revision-0 session having a different rendered page count/geometry and asserts the first status projection is unavailable, then becomes correct after publication. **Required next state:** `in-progress` through an orchestrator-created WO-002 fix order; no WO status was changed here.

- **B — Noninteractive text still owns an egui hover response — Severity: blocker.** Evidence: [`render_item()`](../../../../apps/scriptcraft/src/status_bar.rs:98) renders `StatusItemKind::Text` through `ui.label()`. In the pinned egui 0.29.1 implementation, a default `Label` allocates a `Response` with `Sense::hover()`; the `None` return from `render_item()` merely hides that response from this caller and does not make the widget response-free. The design/acceptance requires noninteractive text to own no pointer response (and the desktop blueprint explicitly says response-free). The current test at [`status_bar.rs`](../../../../apps/scriptcraft/src/status_bar.rs:643) only asserts `None` from `render_item`, so it does not prove the criterion. **Recommendation / precise fix scope:** render noninteractive status text with no pointer-sensing response (for example, a painter-only text path or a label configured with empty sense if that preserves intended accessibility), and extend the test to inspect/establish absence of pointer hover/click/focus/cursor ownership while retaining labels and readable text. Interactive capability behavior should remain unchanged. **Required next state:** `in-progress` through the same orchestrator-created WO-002 fix order; no WO status was changed here.

### Warnings

- **Native desktop visual/input smoke unavailable — Severity: warning.** This validation surface has no native-window screenshot/input automation. No live normal/narrow window or native keyboard/pointer smoke was performed or claimed. The manifest says no manual/external hard-stop is configured; the WO and initiative classify smoke as recommended, so this limitation alone is not a blocker.

### Notes

- The implementation record reports an initial required focused test failure: diagnostic assertion expected `layout-cache-hits=6` but `safe_summary()` emits `cache-hits=6`. The continuation changed the assertion to the actual stable key while preserving privacy/category assertions. It also records earlier formatting/breakpoint/rendered-scroll fixture failures and their repairs. These are retained as history, not hidden or reclassified; the validator's independent reruns all pass below.
- The `ui.label()` pointer-sense finding is based on the exact egui 0.29.1 dependency implementation and the explicit response-free contract, not a general design preference.
- No deceptive, backdoor, malicious, nefarious, or otherwise rogue behavior was found. The implementation stays within the authorized app/requirements/desktop-blueprint scope; WO-003 zoom, core modifications, and unrelated changes are absent.
- Changed durable documents match the WO's `update` classifications: the additive `REQ-WRITING-LOOP-006` criteria and desktop status/page ownership contract are present; feature, technical-context, and core blueprints remain unchanged.
- Desktop rendered geometry tests are deterministic egui integration/unit tests, not a substitute for native-window observation. The optional smoke limitation is explicit above.

## Required command results (fresh validator reruns)

All commands below were run against live `HEAD` at the target commit. Each exited 0.

| Command | Fresh outcome |
|---|---|
| `cargo fmt --all -- --check` | Exit 0; passed. |
| `cargo test -p scriptcraft status_bar_` | Exit 0; 13 passed, 0 failed, 78 filtered out. |
| `cargo test -p scriptcraft visible_page_` | Exit 0; 1 passed, 0 failed, 90 filtered out. |
| `cargo test -p scriptcraft toolbar_` | Exit 0; 28 passed, 0 failed, 63 filtered out. |
| `cargo check --workspace` | Exit 0; passed without warnings. |
| `cargo test --workspace` | Exit 0; 91 app tests + 45 core unit tests + 37 core integration tests = 173 passed, 0 failed; 0 doc tests. |
| `cargo build --workspace` | Exit 0; passed. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Exit 0; passed. |
| `git diff --check fde481db08ba058a1327e2d23abb9b056c2556c3..4be598803fe19c8f212115ae0f2455afef412fac` | Exit 0; committed range has no whitespace errors. |
| `git diff --check` | Exit 0; pre-validation unstaged recordkeeping diff has no whitespace errors. |

The required automated gates pass; the outcome is nevertheless `fail` because explicit implementation criteria are unmet, not because a gate failed. Desktop smoke was unavailable but is optional under policy.

## Decision

- **Outcome:** `fail`
- **Required next state:** `in-progress` after orchestrator reconciliation and creation/dispatch of a narrowly scoped WO-002 fix work order; keep this original WO from being marked complete until the fix passes independent validation.
- **Fix work order required:** `yes — recommend WO-002a-freshness-and-noninteractive-text.md` covering blockers A and B only.
- **State boundary at validator completion:** This validator did not create the fix order, change WO-002 status, change initiative phase/status, stage/commit, or start WO-003. Orchestrator reconciliation was pending at that time and is recorded below.

## Subsequent orchestrator dispatch reconciliation — 2026-10-28

- The original validation outcome remains `fail` for blockers A and B on the exact comparison range recorded above; the validator's findings and command results are preserved unchanged.
- The linked WO-002a fix was subsequently dispatched to fresh implementer host dispatch ID `01a183b3-777f-7055-b601-e91404620223`. This is a host dispatch identity only; no internal session/agent ID is inferred. Its reported implementation checks pass and its status is `in-review`; no fresh fix validation or completion is claimed here.
- Original WO-002 remains `blocked` until an independent fresh validator passes WO-002a and the full original acceptance boundary. Initiative remains `execute` / `active`; WO-003 remains `draft` and undispatched. The stable fix commit is pending at the time of this reconciliation.

## Subsequent fix-commit reconciliation — 2026-10-28

- WO-002a implementation commit `a73c9de14195fb32ae36dce3ae2bba090c8970cf` was created directly atop `4be598803fe19c8f212115ae0f2455afef412fac`. Its exact paths are `apps/scriptcraft/src/main.rs`, `apps/scriptcraft/src/status_bar.rs`, and `apps/scriptcraft/src/toolbar.rs`; committed-range whitespace check passed. This records subsequent orchestration only and does not alter this validation's original `fail` outcome or findings.
- WO-002a remains `in-review`; original WO-002 remains `blocked` pending a separate fresh validation of the fix and full original WO-002 acceptance. No validation of the new commit, completion transition, or WO-003 work is claimed.

## Orchestrator reconciliation — 2026-10-28

- **Validator dispatch:** Reconciled actual fresh validator host dispatch ID `01a183a1-84ed-71a8-bc37-8608a4b1633a`; the host does not expose a stable internal session/agent ID, which remains unclaimed.
- **Outcome and comparison retained:** `fail` for blockers A and B against the exact stable committed range `fde481db08ba058a1327e2d23abb9b056c2556c3..4be598803fe19c8f212115ae0f2455afef412fac`. The acceptance findings, failed outcome, warnings, notes, and all historical test attempts/results above are preserved unchanged.
- **Orchestrator state transition:** The initiative phase is reconciled as `execute` → `validate` for this fresh validator dispatch, then `validate` → `execute` after the failed result and fix-order creation; initiative status remains `active`. WO-002 transitions `in-review` → `blocked` pending its linked fix. The narrow fix order [`WO-002a-freshness-and-noninteractive-text.md`](../work-orders/WO-002a-freshness-and-noninteractive-text.md) was created in `draft` and is not yet dispatched. WO-002 remains incomplete; WO-003 remains `draft` and undispatched.

# Initiative: Title-Bar Transition and Elements Alignment — v3 Follow-up

## Metadata and iteration boundary

- **Phase:** `closeout`
- **Status:** `closed`
- **Authoritative version:** `v3` — a new post-v2 follow-up iteration; it does not reopen, revise, or replace closed v1 or v2.
- **Created/updated:** `2026-10-27`
- **Mode:** `standard`; no rapid path was requested.
- **Parent history:** [`titlebar-transition-and-elements-alignment-v1.md`](titlebar-transition-and-elements-alignment-v1.md:1) and [`titlebar-transition-and-elements-alignment-v2.md`](titlebar-transition-and-elements-alignment-v2.md:1) are closed historical iterations. Their source/API evidence and completed work are inputs, not proof that these v3 defects are fixed.
- **Allowed artifacts:** this v3 plan and exactly the two v3 work orders listed below. No source change, design review, validation result, or implementation is created by this planning session.

## User-confirmed baseline and outcome

The user's two runtime observations are authoritative acceptance baselines, including when a planning environment has no usable display:

1. A real black bar a few pixels high exists directly beneath the toolbar, persists after deleting the target folder and rebuilding from scratch, and must be removed.
2. Double-click maximize/restore visibly makes toolbar icons/text swell or shrink during native animation and snap to the correct size at the end. The fix must stabilize visual metrics during the transition, not merely report that native smoke is unavailable.

**Single v3 outcome:** after implementation, the app-owned toolbar bottom has no black strip, and toolbar icon/text/page presentation metrics remain visually stable throughout real maximize and restore animation while preserving existing title, Elements, cursor, popup, input, and 128-point contracts.

## Direct-translation confirmation

- **Confirmed:** the user explicitly supplied the two defects, exact v3 work-order IDs/filenames, implementation direction, prohibited approaches, deterministic-test expectations, and mandatory user visual gates. This is the required broad-prompt direct translation confirmation under [`02-initiative.md`](../../../process/02-initiative.md:159).
- **Reduction made:** v3 contains exactly two focused WOs. The prior Elements cursor work, native research, and integrated reconciliation remain closed evidence and are not reopened.
- **Acceptance policy:** deterministic tests are necessary but cannot complete either WO. Each WO requires a user visual verification checkpoint on the actual running UI; lack of a display is not permission to pass the known bar or snap-and-zoom defect.

## Branch/base evidence

- Prior v2 recorded base `master`, HEAD `e77d6c4ee424403c5ab779d372745a51e4cafd4e`, implementation commit `44f452a`, and merge commit `2ab1eac`; v2 closeout recorded the final checkout as clean on `master`.
- This planning session did not run Git commands and therefore makes no stronger current branch, HEAD, or cleanliness claim. Before implementation dispatch, the orchestrator must verify the current base and create/record the required initiative branch under project policy.
- **Implementation gate:** design review and human design approval remain pending; no implementer may dispatch from this plan until those gates and branch evidence are recorded.

## Scope

### In scope

- Trace and correct the actual black strip through panel/frame/toolbar height, bottom padding, central-panel placement, fill/stroke/separator, clipping, and logical-to-physical rounding.
- Replace the live width-derived transition metric behavior with an app-owned stable presentation policy and explicit transition state/fallback, without global egui zoom or unsupported native polling.
- Add deterministic pure geometry/physical-pixel and metric/state tests, preserving the existing title/toolbar/Elements/cursor/popup/input contracts.
- Require user visual verification after implementation for the black-bar disappearance and during real maximize and restore animations.

### Out of scope

- Reopening or changing closed v1/v2 artifacts, Elements cursor behavior, popup alignment, semantic editing, persistence/export, broad theme redesign, full-screen, or native animation-control promises.
- Using `has_requested_resize`, desktop/work-area polling as authority, global egui zoom, blind paint-over, or silently accepting unavailable native evidence.
- Creating a design review, validation artifact, source change, or implementation evidence in this session.

## Current source and installed API findings

- [`update()`](../../../../apps/scriptcraft/src/main.rs:2163) uses one exact-height `TopBottomPanel`; [`chrome_frame()`](../../../../apps/scriptcraft/src/main.rs:2061) supplies the app fill with no stroke; the panel separator is disabled.
- [`toolbar_content_height()`](../../../../apps/scriptcraft/src/main.rs:2095) is two interaction heights plus two points of bottom padding; [`toolbar()`](../../../../apps/scriptcraft/src/main.rs:1087) allocates the row and then the bottom padding. [`chrome_surface_ledger()`](../../../../apps/scriptcraft/src/main.rs:123) already models the panel, frame, title, row, bottom padding, central panel, clip, and physical rows, but prior v2 tests did not prove ownership of the user's observed pixels.
- [`live_document_surface()`](../../../../apps/scriptcraft/src/main.rs:1411) consumes [`presentation_metrics_with_policy()`](../../../../apps/scriptcraft/src/main.rs:1900), whose only current policy is `LiveWidthDerived`; width changes can therefore change page scale and renderer font during native animation.
- The app declares eframe `0.29` in [`apps/scriptcraft/Cargo.toml`](../../../../apps/scriptcraft/Cargo.toml:8), resolving to the installed eframe/egui/egui-winit `0.29.1` and winit `0.30.13` evidence recorded in v2. [`ViewportInfo::maximized`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/data/input.rs:186) is an optional observed snapshot; [`ViewportCommand::Maximized(bool)`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/viewport.rs:937) is a request. No supported transition progress/timeline or `has_requested_resize` authority exists.
- egui global scale APIs affect layout, input conversion, fonts, IME, and accessibility; they are not acceptable as a page-only transition lock. The stable policy must be app-owned and safe when transition progress is unavailable.

## Requirements and blueprints

- [`REQ-WRITING-LOOP-005`](../../../requirements/features/screenplay-writing-loop.md:90) — consistent desktop command toolbar without semantic editing changes.
- [`AC-WRITING-005.5`](../../../requirements/features/screenplay-writing-loop.md:115) — deterministic toolbar widths, gaps, clipped edge, and the typed 128-point Elements override.
- [`AC-WRITING-005.6`](../../../requirements/features/screenplay-writing-loop.md:131) — compact title strip and known-state maximize/unmaximize fallback.
- [`AC-WRITING-005.7`](../../../requirements/features/screenplay-writing-loop.md:137) — arrow cursor ownership and document I-beam ownership.
- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:83) — toolbar allocation, row/clip edge budget, 128-point Elements baseline.
- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:105) — native presentation hints and bounded viewport request semantics.
- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:113) — unified app-chrome surface, fill, zero inset, and no separator contract.

## Work-order decomposition and dependency

1. [`V3-WO-001-toolbar-bottom-black-bar-removal.md`](work-orders/V3-WO-001-toolbar-bottom-black-bar-removal.md:1) — first; directly removes the confirmed app-visible strip and establishes the corrected chrome coverage/physical-row ledger.
2. [`V3-WO-002-stable-titlebar-transition-metrics.md`](work-orders/V3-WO-002-stable-titlebar-transition-metrics.md:1) — second; consumes the stable chrome geometry and independently stabilizes presentation metrics during maximize/restore.

The order is sequential. WO-002 must not use a changed toolbar geometry as an unexplained metric input, and WO-001 must not broaden into transition behavior. Each WO receives a separate fresh implementer session and separate fresh validation session under the Forge process. No v3 validation files are created now.

## Durable-document and approval policy

- Default classification for both WOs is requirements `verify unchanged`: existing acceptance criteria remain accurate unless implementation proves a stronger user-facing contract.
- Default blueprint classification is `verify unchanged`; update only for an evidenced durable app-owned surface or stable presentation contract, with the smallest authorized wording change.
- The 128-point Elements baseline, prior cursor/popup/title behavior, known-state viewport request semantics, and no-exact-restore claim are preserved unless a separately evidenced in-scope correction requires clarification.
- Pending: design review, human design approval, implementation, per-WO validation, actual user visual verification, branch/base confirmation, and closeout.

## Design-readiness and acceptance gates

- **Design gate:** pending fresh alignment review; this plan is not implementation authorization.
- **Human gate:** pending explicit human design approval after review.
- **WO-001 visual gate:** the user must run the rebuilt app and visually confirm the black strip is absent directly below the toolbar across a normal frame and a maximize/restore cycle. If the strip is still present, the WO fails. If app-owned paths are exhausted and the implementer proves the pixels are outside app ownership, the WO returns `blocked` for the user; it does not pass as unavailable.
- **WO-002 visual gate:** the user must double-click to maximize and double-click to restore while watching the real animation, and confirm toolbar icons/text and page presentation metrics do not swell or shrink before settling. “Native evidence unavailable” is not acceptance. A platform limitation may be recorded only after the app-owned stable policy is implemented and the user gate is attempted.
- **Validation gate:** deterministic tests, required Cargo checks, diff checks, and the applicable visual checkpoint must be recorded before a WO can be accepted; completion remains the validator's decision.

## Plan transition record

- **Request:** user requested a fresh v3 planning/work-order iteration on 2026-10-27.
- **Transition:** new post-v2 follow-up plan and exactly two v3 work orders created; v1/v2 remain closed.
- **Current state:** `plan` artifacts prepared; review, approval, execution, validation, and closeout are pending.

## Append-only closeout disposition — terminated historical v3 iteration (2026-10-27)

This is the current v3 disposition and supersedes the earlier “Current state” summary above without rewriting the recorded plan, failed gates, implementation evidence, or validation findings. At the user's direction, the incomplete v3 diagnostic lineage is terminated and tabled; this does **not** represent its failed or incomplete experiments as successful. The initiative is in `closeout` / `active` pending the separately dispatched Git operation and its evidence. It must not be marked `closed` before that evidence is recorded.

### Consolidated work-order and validation checklist

- [x] **V3-WO-001:** initial black-bar implementation's user visual gate failed; its earlier work-order metadata saying `complete` is not treated as acceptance. It is superseded by the validated correction below.
- [x] **V3-WO-001a:** failed the user visual gate; black strip remained and toolbar text/content was occluded. Superseded by V3-WO-001b; not complete.
- [x] **V3-WO-001b:** preserve its independent `pass` validation and `complete` disposition. The validated black-bar correction restored the 2-point padding, matched the renderer clear color to the chrome surface, and passed the user's clean-build normal/maximized/restored visual gate. This is the accepted black-bar result; its validation remains historical evidence and is not reopened here.
- [x] **V3-WO-002:** failed the user's real maximize/restore visual gate; the app-owned metric policy did not prevent swelling, shrinking, and snapping. Not complete; tabled/terminated at the user's direction.
- [x] **V3-WO-002a:** the native-titlebar A/B attempt did not visibly expose native chrome; failed visual experiment, not complete; tabled/terminated.
- [x] **V3-WO-002b:** the native-titlebar mode-wiring attempt still showed hidden/transparent native chrome; failed visual experiment, not complete; tabled/terminated.
- [x] **V3-WO-002c:** the startup-handoff attempt again failed the user's native-titlebar visibility gate. Not complete; tabled/terminated. Its artifact metadata says `plan`, while its appended implementation evidence and the execution log say `in-review`; this conflict is preserved, not silently reconciled or used to imply completion.
- [x] **V3-WO-002d:** implementation and automated checks were recorded, but the required user-visible native-chrome gate and fresh validation were not recorded. It is incomplete, not a visual pass; tabled/terminated. Its artifact metadata says `plan`, while its implementation addendum and the execution log say `in-review`; this conflict is preserved, not silently reconciled or used to imply completion.
- [x] No V3-WO-002/002a/002b/002c/002d validation is asserted to exist or pass. The V3-WO-001b validation is retained as the sole accepted result in the terminated v3 fix chain.

### Follow-up and closeout boundary

- **Deferred objective:** seamlessly integrate ordinary native macOS chrome by removing native title text beside the traffic lights and eliminating the distinct titlebar color band, while preserving usable native controls/toolbar and the observed no-swell/no-shrink/no-snap maximize/restore behavior.
- **Owner / next action:** successor initiative owner (orchestrator) carries the unapproved [`V4-WO-002` proposal](work-orders/V4-WO-002-native-titlebar-seamless-chrome.md:1) into a new initiative only after v4 closeout. That initiative requires a fresh design review, explicit human approval before implementation, and a strict clean-build visual gate on macOS. No successor initiative is created by this closeout preparation.
- [x] **Git closeout:** completed after fresh inventory. Commit `053a581d4a180cdc2cbe34b03470c639d08c024b` was fast-forwarded into `master`; the initial post-merge tree was clean. Full terminal evidence is recorded below and in the shared execution log.
- [x] User authorization is recorded: commit the current entire cleanly inventoried cumulative tree and merge the continuation branch into `master`. This is authorization, not evidence that inventory, staging, commit, or merge has occurred.

## Terminal closeout evidence — v3 (2026-10-27)

- **Phase/status:** `closeout` / `closed`. The incomplete v3 diagnostic lineage is terminated, not represented as fully implemented. V3-WO-001b remains the accepted validated black-bar correction; failed and incomplete v3 diagnostics remain tabled/terminated as itemized above.
- **Cumulative initiative commit:** `053a581d4a180cdc2cbe34b03470c639d08c024b` (`Close out terminated v3/v4 titlebar diagnostics`), created on `dev/2026-10-27-titlebar-transition-and-elements-alignment-v3` from base `6a6739c5263edee9896b32c00fc6f8470e35f3d6`.
- **Merge evidence:** fast-forward merge into `master`; no separate merge commit was created. `master` advanced from `6a6739c5263edee9896b32c00fc6f8470e35f3d6` to `053a581d4a180cdc2cbe34b03470c639d08c024b`, which is both the initiative commit and merged tip.
- **Inventory/verification:** exactly 18 initiative paths were committed: cumulative `apps/scriptcraft/src/main.rs`, the shared execution log, the v3/v4 plans, reviews, validations, and work-order artifacts (including the V4-WO-002 proposal). No unrelated or secret untracked files were present. Ignored `.DS_Store`, `.reference/.DS_Store`, and `target/` build output were not staged. `git diff --cached --check` passed.
- **Terminal branch evidence:** after the fast-forward, `git status --short --branch` reported `## master` with no changed paths; `HEAD` and `master` both resolved to `053a581d4a180cdc2cbe34b03470c639d08c024b`. The separate final closeout-record commit is made on `master`; post-commit terminal verification is reported by the closeout execution session.
- **Successor boundary:** no successor initiative was created. The unapproved V4-WO-002 proposal remains deferred for a separately controlled initiative.

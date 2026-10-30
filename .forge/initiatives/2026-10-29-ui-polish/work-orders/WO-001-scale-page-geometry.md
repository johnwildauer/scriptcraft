# Work Order: WO-001 — Scale and Page Geometry

## Metadata

- **Status:** `complete` — independent validation passed; WO-001 dependency is satisfied.
- **Assignee:** Desktop UI implementer, separate coding-agent session; host/session identifier unavailable in this interface and not inferred.
- **Phase/order:** First feature work order; sequential execution.
- **Initiative:** [UI Polish plan, including subsequent investigation and user-confirmed handoff](../ui-polish-v1.md#subsequent-full-scope-investigation--2026-10-29).
- **Generated filename:** [WO-001-scale-page-geometry.md](WO-001-scale-page-geometry.md).
- **Owner/orchestrator:** Forge orchestrator; implementer owns this work order's desktop changes and evidence.
- **Validation:** [validation-WO-001.md](../validations/validation-WO-001.md) — `pass`; independent validation recorded 2026-10-29.
- **Fixes:** none. **Implementer session/agent ID:** unavailable in this interface and not inferred. **Session exception:** no combined-WO exception; separate implementation session used.

### Execution authorization record — 2026-10-29

- The user explicitly authorized implementation of all WOs and directed the initiative to `execute`; WO-001 was implemented as written, including its proposed 360-logical-point 100% baseline and temporary requirements/blueprint deferral to WO-005.
- The formal design-review artifact remains `blocked`; no review pass or independent design approval is claimed. The direct user authorization is the basis for proceeding.
- The manifest's per-WO cadence still requires a separate fresh validator immediately after implementation evidence is complete. This implementer will record `in-review`, not `complete`, and will not author a validation verdict.

## Summary

Replace responsive page fitting with fixed, independently zoomable page presentation and correctly bounded two-axis scrolling. Apply native UI scaling exactly once, with a 2.3 fallback only when the native scale is missing or invalid. Preserve editing, source, pagination, PDF, and status behavior. The 360-point visual baseline was initially an agent-default proposal; the user explicitly authorized this calibration for execution in the record above. It is not a measured or OS-validated visual result.

## Dependencies and gates

- No predecessor work order in this initiative. The existing desktop app, its core layout/export boundary, the locked egui/eframe stack, and current status zoom controls are prerequisites. No new library dependency is planned.
- Before dispatch, the orchestrator must reconcile the overlapping active [status-bar initiative](../../2026-10-28-status-bar/status-bar-v1.md), actual ancestry and uncommitted work; verify the current worktree and required initiative branch non-destructively. The plan records successful branch creation but also contains stale contradictory pending language; this draft does not independently verify current Git state. Preserve unrelated changes.
- The normal review gate requires independent design review and human approval. The formal review remains `blocked`; the user explicitly authorized execution notwithstanding that state, as recorded above. This direct authorization is not a review pass. Use a separate fresh validator immediately after this evidence package is complete.
- Later status/appearance, Sprint Timer, and Focus Mode work must integrate sequentially with the resulting zoom/editor seam; do not implement them here. Draft [WO-005](WO-005-durable-document-reconciliation.md), owned by the orchestrator and a separately dispatched documentation implementer, depends on completion of the feature work orders and must reconcile durable requirements and desktop blueprint against actual results. WO-005 is not approved; no unrelated sibling uses a fix suffix.
- The WO-005 durable-document deferral is explicitly authorized by the user's direction to execute WO-001 as written. It is recorded as a temporary discrepancy, not a policy waiver or a claim that the durable documents are accurate. The independent validator must compare to the authorized WO and record the discrepancy; WO-005 remains the reconciliation owner.

## In scope

- Desktop native/effective scale resolution, fallback, monitor changes, fixed sheet presentation, zoom/pinch integration and viewport geometry.
- Actual scroll content and inner clip, explicit 16-point exterior horizontal buffer, centered sheet, horizontal overflow and offset/clamp, resize and independent vertical page navigation.
- Preserve single editor input ownership, transforms, caret/selection/IME and visible-page freshness; add focused executable headless and rendered-frame tests and record evidence.

## Out of scope

- Status integration redesign, appearance/theme, Sprint Timer, Focus Mode, notification and fullscreen behavior; later feature groups own these.
- Core page profile, pagination, screenplay semantics, persistence schema, PDF formatting, or user preferences; no page-zoom persistence. Do not alter native window chrome or toolbar layout.
- Editing the initiative plan, requirements, blueprints, other work orders or review artifacts as part of this implementation. Reconciliation is deferred to planned WO-005 as described below. No review, implementation, branch change, commit, merge or user regression is performed by authoring this draft.

## Existing applicable requirements — verbatim

The following are current text from [screenplay writing loop requirements](../../../requirements/features/screenplay-writing-loop.md). The responsive baseline and 2.0 fallback in 006.7/006.10 **conflict** with this proposed work order; they are quoted unchanged for traceability, not misrepresented as already updated. Subject to design approval, the new acceptance below supersedes only those conflicting parts for this initiative until planned WO-005 reconciles the durable text.

> **AC-WRITING-001.2:** Given an element, the system shall preserve its explicit role and canonical UTF-8 text independently of capitalization, spaces, tabs, indentation, or font appearance.
>
> **AC-WRITING-001.3:** Given a valid selection, the system shall support insertion, deletion, split, join, role change, undo, and redo while preserving valid text boundaries and element order.
>
> **AC-WRITING-002.1:** Given the same project and fixed page profile, the system shall derive stable page structure and source mappings independent of window zoom.
>
> **AC-WRITING-002.4:** Given a document position and rendered geometry, the system shall support a safe caret/hit-test round trip at valid text boundaries.
>
> **AC-WRITING-003.1:** Given a valid project, the system shall serialize a versioned native representation containing semantic content, stable IDs, metadata, and page-profile inputs, while excluding transient caret/layout/undo state from required content.
>
> **AC-WRITING-004.1:** Given a project, PDF export shall use fixed print layout rather than transient viewport layout.
>
> **AC-WRITING-004.2:** Given a PDF or plain-text export, the system shall not silently replace the native source of truth or falsely mark the native project clean.
>
> **AC-WRITING-005.7:** Toolbar action controls and the Elements control/label surface shall request the standard arrow cursor, while the existing whole-document editing response shall request the text/I-beam cursor. Decorative labels and painted page rectangles shall not create competing cursor owners.
>
> **AC-WRITING-006.4:** At narrow widths, a deterministic bounded fallback shall retain access to visible-page state, clean/unsaved state, operation/error status, diagnostics, and zoom controls. Status text shall clip horizontally as single-line content rather than wrap or grow the footer.
>
> **AC-WRITING-006.5:** Given the actual rendered sheet rectangles and actual editor scroll clip, the visible-page Item shall use one-based numbering and select the sheet with the greatest vertical overlap; exact overlap ties shall select the earlier rendered sheet. If no sheet overlaps, it shall select the nearest vertical interval, with exact-distance ties selecting the earlier sheet. The total shall include every rendered sheet, including an optional title page.
>
> **AC-WRITING-006.6:** Visible-page state shall be independent of caret/source position and remain current after scrolling, resize, presentation-scale changes, layout reflow, and conditional caret reveal. If layout or geometry is invalid, missing, or stale, the status bar shall show an explicit unavailable state until fresh geometry is published. Page-status updates shall not scroll the editor, reveal the caret, steal focus, or change editor input ownership.
>
> **AC-WRITING-006.7:** The desktop status bar shall provide one accessible, apparently continuous whole-percent page zoom slider from 75% through 300% and an adjacent menu containing exactly 75%, 90%, 100%, 125%, 150%, 175%, 200%, 250%, and 300%. Both controls shall share one transient per-window value and show every intermediate integer accurately. 100% shall retain the existing responsive window-fit page baseline, with the selected percentage retained across resizes; zoom shall scale only page presentation and allow horizontal as well as vertical navigation without changing editor source mapping or focus/IME ownership. Zoom shall not persist or alter native project data, session revision, saved/dirty or undo state, fixed pagination/source ranges, or fixed-layout PDF output, and shall not globally scale the toolbar/status controls.
>
> **AC-WRITING-006.10:** A supported native inward/outward pinch event shall adjust the same transient per-window whole-percent page zoom as the status slider and presets, preserving the inclusive 75–300% limits and all document, session, pagination, export, and editor-input invariants in AC-WRITING-006.7. Inward pinch is the negative direction of the backend's supported zoom event; delivery is platform/backend dependent and no distinct squeeze event is promised. Effective egui UI pixels-per-point shall use a valid OS-native value once when reported, or exactly 2.0 when unavailable/invalid; this UI scale is independent of page zoom and shall not multiply native scale twice.

## Blueprints and inspected evidence

- [Desktop container contracts](../../../blueprints/containers/scriptcraft-desktop.md#system-contracts): single editor response and conditional reveal; actual page/clip status geometry; zoom, pinch, and native scale. Its responsive-fit and 2.0 statements are temporarily inconsistent with the proposed result and require WO-005 reconciliation. Other contracts remain in force.
- [Writing-loop feature composition](../../../blueprints/features/screenplay-writing-loop.md#integration-contracts): one fixed-profile source/layout boundary for desktop and PDF; zoom is presentation-only. [Core fixed-profile facts](../../../blueprints/components/screenplay-core.md#wo-004-verified-spacing-and-export-facts): 612 × 792 print points and profile-derived geometry remain unchanged.
- [Desktop page surface](../../../../apps/scriptcraft/src/main.rs#L495) currently computes a width from outer available space and puts leading and trailing space in a horizontal row; [metrics](../../../../apps/scriptcraft/src/main.rs#L959) fit a 576-point nominal maximum to width, and the pre-input hook consumes pinch. [Scale helpers](../../../../apps/scriptcraft/src/status_bar.rs#L8) currently use 2.0 fallback. These facts do not establish a reproduced scrollbar defect: row gaps, reserved scrollbar width, clip margins and animated/overlay bars are hypotheses to measure.
- [App manifest](../../../../apps/scriptcraft/Cargo.toml#L8) specifies eframe 0.29; [lockfile](../../../../Cargo.lock) pins the relevant eframe, egui and egui-winit versions to 0.29.1. Cached [egui context](../../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/context.rs#L477) multiplies UI zoom by reported native pixels per point, assuming 1.0 when absent. Cached [egui-winit input](../../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-winit-0.29.1/src/lib.rs#L235) supplies the window scale and updates it on scale-factor changes. Cached [scroll area](../../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/containers/scroll_area.rs#L538) uses previous bar state and animation to allocate inner space; [its end pass](../../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/containers/scroll_area.rs#L818) measures content and computes overflow. These cached paths are local inspection evidence, not portable project dependencies.

## Implementation plan

1. Reinspect current code, prior work and actual locked framework behavior. Instrument or expose test-only observed sheet bounds, content size, effective inner rectangle/clip, bar reservation/visibility, item spacing and scroll offsets. Establish whether row gaps, bar feedback or another cause actually produces overflow; do not assert a cause merely from inspection.
2. At the existing pre-input boundary resolve each incoming viewport's finite positive native factor N. A reported 1.0 is valid, not missing. For valid N set UI zoom to 1.0 and effective pixels per point E to N; for absent, zero, negative, NaN or infinite N sanitize the input and use UI zoom 2.3, so egui's absent-native assumption gives E = 2.3. When a valid native factor arrives or changes, replace the fallback immediately; never retain an old factor or multiply native twice. Account for actual framework input/screen-rectangle timing and invalidate stale page/status geometry on changes. Native scale is not physical monitor DPI in inches.
3. For selected page percentage Z, use a stable nominal 100% page width of 360 logical points under the above policy, or equivalently 360 × resolved scale / E logical points, multiplied by Z/100. Derive height, font metrics, painting, hit tests, selection, caret and IME from the same uniform profile-to-page transform; no second native-scale or historic 1.6 multiplier. Retain the profile's aspect ratio and the existing transient whole-percent 75–300 range, exact preset order and accessible control behavior. Page zoom is not global UI zoom. Consume each supported pinch event once and keep the fractional accumulator synchronized when controls change it.
4. Use the actual available scroll inner clip width V after applicable parent clipping and reserved vertical-bar space, not outer width. For page width P and B = 16 logical egui points, target content width C = max(V, P + 2B) and sheet left = (C − P)/2 in content coordinates. At zero horizontal offset, fitting pages are centered within the inner viewport; overflowing pages retain B on each side and are horizontally reachable. Avoid implicit row item gaps in measured bounds. Overflow iff P + 2B > V; equality fits, allowing only a documented tolerance strictly below one physical pixel for rounding. Handle zero/tiny viewports safely. Stabilize actual bar/inner-width feedback over frames for reserved and overlay/animated configurations, rather than assuming a bar always consumes width. Do not force a horizontal bar when the page plus buffers fits.
5. On zoom or resize preserve an existing horizontal offset when still valid; otherwise clamp to [0, max(0, C − V)], including zero when content fits. Verify final painted positions as well as returned post-frame state because egui can update scroll state after content is laid out. Preserve vertical scrolling, page gaps and nearest conditional caret reveal; passive scrolling must not be overridden. Publish fresh actual page rectangles and clipped viewport, invalidating stale visible-page snapshots across scale/size/scroll/reveal changes. Retain pointer/source inverse mapping, title-page handling, focus, IME and accessibility behavior.
6. Update old tests that encode responsive fit or 2.0 fallback, without removing their other assertions. Add focused deterministic tests and rendered egui-frame/input tests specified below; preserve all manifest checks and collect evidence for independent validation.

## Files and systems

- **Likely update:** [desktop main and tests](../../../../apps/scriptcraft/src/main.rs), [status zoom/scale helpers and tests](../../../../apps/scriptcraft/src/status_bar.rs); add a small desktop-local geometry module/test file only if needed. No new dependency expected.
- **Read/verify unchanged:** [core layout](../../../../crates/scriptcraft-core/src/layout.rs), [editing](../../../../crates/scriptcraft-core/src/editing.rs), [exports](../../../../crates/scriptcraft-core/src/exports.rs), and project serialization; verify actual outputs rather than assume parity.
- **Avoid changing:** durable requirements and blueprints, shared toolbar composition, unrelated UI features, project schema, generated review/validation artifacts and Git state. Implementation reporting later follows Forge process in this work order and the execution log; authoring this draft does not edit the log.

## Validation and acceptance

### Executable focused and rendered tests

Add named automated tests with the `wo001_` prefix so `cargo test -p scriptcraft wo001_` executes the new focused suite; record the number of tests and results, not just a successful zero-test command. Use real app geometry and egui frames/input, not arithmetic-only stand-ins. Update and retain existing relevant tests and run the full workspace suite.

1. **Scale and transitions:** Drive the actual pre-input hook and egui frames with native factors 1.0, 1.5, 2.0, absent, zero, negative, NaN and infinity, then absent → 1.5 → 2.0 → absent. Assert finite effective values, UI factors and immediate fallback replacement. At 100%/150%, assert logical/physical sheet widths respectively: 360/540 and 540/810 at 1.5; 360/720 and 540/1080 at 2.0; 360/828 and 540/1242 for missing native 2.3. Check height from profile aspect ratio. Include 1.0 as known and assert toolbar/status are not enlarged by changing page zoom. Use measured effective pixels per point, not native factor twice.
2. **Zoom:** Exercise the actual slider, keyboard and presets and injected supported inward/outward pinch, intermediate whole percents, fractional accumulation, invalid events, bounds and control-to-pinch synchronization. Verify the event is consumed once and neither global zoom nor editor text input receives it. Assert page width/height and font/geometry transforms for 75%, 100%, an intermediate value, 150% and 300%, including resize with percentage retained.
3. **Measured geometry:** In rendered multi-frame tests record actual inner clip, page rectangles, content bounds, scrollbar use, item gaps, horizontal offset and max offset. Cover wide fit, exact threshold, just above/below threshold by more than rounding tolerance, narrow overflow, multi-page vertical overflow, vertical bar present/absent, supported reserved and overlay bar styles and animation settling. Verify both 16-point buffers, centered fit, no spurious horizontal scroll, both overflow edges reachable, no hidden gap-induced excess, no sustained bar/content oscillation and stable final output over repeated frames. Specify the physical-pixel rounding tolerance used. Test at more than one native scale and near thresholds based on measured inner, not assumed outer, width.
4. **Scrolling and resize:** Inject horizontal and vertical wheel/scroll input in multi-page frames. Assert unchanged page dimensions on width-only resize; valid horizontal offset is retained, excessive offset clamps after wider resize or zoom reduction, and fit resets to zero. Verify painting converges to clamped positions, vertical navigation remains usable, and page/status calculations use fresh geometry. Test repeated narrow/wide resize and both directions of zoom, not just initial layout.
5. **Editing and invariants:** With a Unicode, multi-page fixture and optional title page, compare project serialization, source ranges/page structure, session revision/dirty state, selection, undo/redo availability and deterministic PDF bytes before and after passive zoom, pinch, resize, scrolling and native-scale changes. Exercise actual rendered pointer-to-caret round trips at several scales and offsets, range selection, keyboard insertion and IME preedit/commit through the one focused editor response. Verify conditional nearest reveal when outside the actual clip, no reveal or focus theft from passive scroll/status, and unavailable-to-fresh visible-page updates with correct overlap/tie/title-page behavior. Distinguish intentional edits from passive invariance assertions.

### Required project-policy commands

Run and record exact command, result and relevant failures for each, in addition to the focused suite:

```sh
cargo test -p scriptcraft wo001_
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check --workspace
cargo build --workspace
```

These are required even though the user owns desktop regression. Record failures and remediation; an unavailable required gate is a blocker, not a pass. Capture changed/created/deleted files, comparison diff or equivalent, tests and outcomes in the work order and execution log before setting it to `in-review`; the fresh validator alone determines completion.

### User-owned post-implementation native regression

The user performs this checklist after implementation; it is not performed or claimed by this draft or the implementer. The orchestrator records actual results or unavailability and reconciles cross-feature evidence at normal validation/closeout. The manifest currently has no configured desktop manual hard-stop; do not invent one, but escalate a behavior that cannot be meaningfully validated through the existing review/policy process.

- Launch the native app and resize across wide, exact-fit and narrow windows at 75%, 100%, intermediate and 300%; inspect fixed sheet size, centered fit, 16-point exterior spacing, no false horizontal bar or flicker, usable two-axis scrolling, and offset behavior after resize/zoom.
- On available monitors with differing native scales, move the window between them and back; inspect single scaling, stable logical page size, readable controls, pointer accuracy, status freshness and no transient stuck fallback. Record monitor/platform limitations instead of claiming coverage.
- On a supported device test pinch in/out and slider/preset agreement; record unsupported event delivery as unavailable. With a multi-page document, type/select, scroll, invoke caret reveal, and exercise actual IME composition after zoom and scrolling. Check visible-page indication, title page if present, keyboard focus, save/reopen and PDF appearance remain correct.

## Durable-document impact

- **Requirements impact:** `update` needed, **deferred to planned WO-005**, not authorized in this WO. [AC-WRITING-006.7 and 006.10](../../../requirements/features/screenplay-writing-loop.md#req-writing-loop-006-desktop-status-bar-and-visible-page) presently specify responsive fit and 2.0 fallback. On implementation, they will temporarily disagree with the fixed 360-logical-point baseline and 2.3 missing-scale fallback. WO-005 must reconcile those exact clauses, preserve the remaining requirements above, and record actual evidence and revision history.
- **Blueprint impact:** `update` needed, **deferred to planned WO-005**, not authorized in this WO. The [desktop blueprint zoom and scale contracts](../../../blueprints/containers/scriptcraft-desktop.md#system-contracts) likewise describe responsive fit and 2.0 fallback; WO-005 must reconcile them with actual fixed page/inner-scroll geometry and single native scaling, without claiming native smoke or other features passed. Verify [feature composition](../../../blueprints/features/screenplay-writing-loop.md#integration-contracts) and [core fixed profile](../../../blueprints/components/screenplay-core.md#wo-004-verified-spacing-and-export-facts) remain correct; do not alter the print profile.
- **Owner and trace:** Orchestrator tracks the explicit interim discrepancy from WO-001 implementation through separate WO-005 completion; the later documentation implementer owns the edits, and its fresh validator checks them. Any material deviation discovered during implementation requires routing through design/policy rather than presenting the documents as updated or accurate. No durable change is authorized merely by this draft.

## Implementation evidence — 2026-10-29

- **Changed product source:** `apps/scriptcraft/src/main.rs`, `apps/scriptcraft/src/status_bar.rs`.
- **Changed Forge artifacts:** this work order, [initiative plan](../ui-polish-v1.md), and [execution log](../execution-log.md). No requirements or blueprint files were changed; the temporary discrepancy to WO-005 remains visible.
- **Comparison evidence:** read-only source `git diff` against `HEAD` plus `git diff --check` (passed). `git status --short --branch` confirmed branch `dev/2026-10-29-ui-polish`; source files were clean before implementation. The pre-existing modification to `.forge/process/04-work-order.md` and untracked `.reference/egui-helpers.md` were preserved. No commit or branch operation occurred.
- **Focused suite:** exact command `cargo test -p scriptcraft wo001_` — passed, 8 passed / 0 failed. Tests exercise fixed-width zoom, native 1.0/1.5/2.0 and missing/invalid 2.3 fallback (including transitions), rendered measured inner-clip threshold/buffers, rendered logical/physical dimensions and aspect ratio, scale-driven visible-page invalidation, and two-axis scrolling/resize/clamping.
- **Project commands:** `cargo fmt --all -- --check` passed; `cargo clippy --workspace --all-targets -- -D warnings` passed; `cargo test --workspace` passed (120 app tests plus 82 core/integration/doc tests; 202 total passed, 0 failed); `cargo check --workspace` passed; `cargo build --workspace` passed.
- **Manifest run command:** `cargo run -p scriptcraft` launched `target/debug/scriptcraft` and remained active for the 10-second liveness interval; it was then stopped with SIGINT. This confirms process startup/liveness only. No screenshot, visual interaction, monitor migration, or OS-specific scaling assertion is claimed.
- **Initial failures and remediation:** an early `cargo test -p scriptcraft` had 117/118 pass because a float assertion used `f32::EPSILON` for `2.3 * 1.5`; the tolerance was corrected to `1e-6`, and the final focused/workspace suites pass. The first `cargo fmt --all -- --check` reported formatting differences; `cargo fmt --all` applied formatter output and the final check passes. Early scroll-test drafts incorrectly expected animated egui wheel scrolling to complete in one frame and treated an end-pinned offset as an interior offset; the rendered regression now advances controlled frames and checks interior-offset preservation, edge reachability, and fit clamping. The final regression passes.
- **Limitations:** native monitor migration, OS-reported scale delivery, and visual appearance were not independently observed. The user-owned native regression checklist remains pending; no user test result is inferred. The expanded acceptance matrix for every scrollbar style/theme/platform configuration was not executed as OS smoke.
- **Fresh validation:** no validation artifact or independent verdict exists yet. The work order is `in-review`, not `complete`.

## Completion checklist

- [x] User execution authorization, review-state exception, branch/worktree evidence, and WO-005 document deferral recorded; formal design-review pass is not claimed.
- [x] Scoped implementation and focused headless/rendered tests completed; actual geometry and scale evidence recorded.
- [x] Required automated commands attempted with outcomes recorded; native OS checklist remains honestly pending.
- [x] Durable-document discrepancy and WO-005 handoff recorded accurately; no unauthorized requirements/blueprint edits.
- [x] Changed files, comparison evidence, available session-identifier limitations, and test results recorded in this work order and execution log.
- [x] Work order was marked `in-review` only after implementation/evidence; separate fresh validation passed and the work order is now `complete`.

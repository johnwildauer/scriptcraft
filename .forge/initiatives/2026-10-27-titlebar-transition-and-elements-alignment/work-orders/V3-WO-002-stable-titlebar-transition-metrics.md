# Work Order: V3-WO-002 — Stable Title-Bar Transition Metrics

## Metadata

- **Status:** `in-review`
- **Assignee:** Fresh desktop transition-metrics implementer
- **Phase/order:** 2 of 2; depends on V3-WO-001
- **Initiative:** [`../titlebar-transition-and-elements-alignment-v3.md`](../titlebar-transition-and-elements-alignment-v3.md:1)
- **Generated filename:** `V3-WO-002-stable-titlebar-transition-metrics.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** pending fresh validation artifact `validation-V3-WO-002.md`
- **Fixes:** none
- **Implementer session/agent ID:** `implementer-fresh-v3-wo002-stable-metrics-2026-10-27T2029Z`
- **Session exception:** none

## Summary / one outcome

Prevent the user's observed snap-and-zoom during double-click maximize/restore by making both toolbar icon/text presentation and app-owned page presentation metrics stable through the observed native transition. Implement an app-owned explicit transition state and deterministic fallback based on last stable metrics plus viewport request/settling observations; do not stop at documenting unavailable native progress.

## Baseline

The user observed that double-click maximize/restore makes toolbar icons/text visibly swell or shrink throughout native animation and snap to the correct size at the end. The required fix targets stable visual metrics during the animation, not merely the final frame and not a native-smoke-unavailable note.

## In scope

- Trace [`title_strip()`](../../../../apps/scriptcraft/src/main.rs:2126), the known-state viewport request, [`presentation_metric_policy()`](../../../../apps/scriptcraft/src/main.rs:1879), [`presentation_metrics_with_policy()`](../../../../apps/scriptcraft/src/main.rs:1900), [`live_document_surface()`](../../../../apps/scriptcraft/src/main.rs:1411), and renderer font/page scale use.
- Implement a stable app-owned presentation transform/metric policy with explicit transition state, last-stable capture, request/settling/release behavior, and safe fallback when eframe cannot expose progress.
- Cover the complete snap-and-zoom acceptance surface: toolbar icon control size, toolbar icon/text font and galley size, toolbar icon/text visual bounds, and page width/scale/font/renderer bounds must be captured and stabilized as one app-owned presentation metric set.
- Keep egui global scale, pointer/input/accessibility/IME semantics, toolbar allocation, title behavior, Elements width, popup behavior, cursor ownership, and semantic layout unchanged.
- Add deterministic policy and metric tests and require user visual verification during real maximize and restore animations.

## Out of scope

- `has_requested_resize` (nonexistent), desktop/work-area polling as authority, native animation duration/progress/interpolation/suppression, platform-specific unsafe hooks, or exact native frame history.
- Global `Context::set_zoom_factor()` / `set_pixels_per_point()` as a page-only lock; toolbar redesign, Elements cursor work, black-bar geometry, and semantic editing.

## Requirements

- **REQ-WRITING-LOOP-005:** “As an individual screenwriter, I want desktop commands in a consistent toolbar, so common project actions are discoverable without changing the semantic editing model.”
- **AC-WRITING-005.6:** “Given the desktop app is running, a dedicated compact title strip shall display centered bold `ScriptCraft` placeholder text above the toolbar. A double-click shall request maximize/unmaximize only when the viewport's maximized state is known; unavailable or ignored state shall use a bounded no-op/native fallback rather than an unsafe or exact-restoration claim that the platform cannot support.”
- **AC-WRITING-005.5:** preserve the deterministic toolbar width/edge contract and typed 128-point Elements width while stabilizing presentation metrics.

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:105) — viewport requests are bounded native hints and restoration is not an exact universal claim.
- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:113) — title/toolbar surface and title hit behavior.
- [`technical-context.md`](../../../blueprints/technical-context.md:31) — presentation scale is above the shared fixed semantic layout.

## Current source/API findings

- [`presentation_metric_policy()`](../../../../apps/scriptcraft/src/main.rs:1879) is currently only `LiveWidthDerived`; [`presentation_metrics()`](../../../../apps/scriptcraft/src/main.rs:1886) derives page width, scale, and font size from available width.
- [`live_document_surface()`](../../../../apps/scriptcraft/src/main.rs:1421) supplies the live width; renderer font and page geometry consume the derived scale. This is the app-owned path to stabilize without changing egui-wide metrics.
- [`title_strip()`](../../../../apps/scriptcraft/src/main.rs:2147) sends `ViewportCommand::Maximized(bool)` only from optional observed maximized state. The command is a request, not a transition acknowledgement.
- Installed eframe/egui/egui-winit `0.29.1` and winit `0.30.13` evidence exposes viewport snapshots and optional rectangles but no supported animation timeline, transition progress, or `has_requested_resize`. macOS may leave maximized state stale after initialization.
- Global egui scale APIs alter layout, input conversion, font selection, IME geometry, and accessibility transforms; they are prohibited for this page-only visual problem.

## Metric boundary and units

The acceptance claim covers toolbar presentation and page presentation together, but it does not claim that all of egui is frozen or that native animation is controlled.

- **Fixed logical egui-point contract:** toolbar allocation widths, toolbar row and title-strip heights, spacing/padding, interaction and hit rectangles, `TOOLBAR_ELEMENTS_WIDTH == 128.0`, popup/ComboBox layout, semantic editor geometry, pointer/input conversion, accessibility geometry, IME geometry, and the global egui `pixels_per_point` value remain ordinary logical egui points. They are not transition metrics and must not be rescaled or frozen through a global egui zoom/pixels-per-point API.
- **App-owned renderer presentation metric contract:** the transition policy owns a local presentation tuple containing toolbar icon control size, toolbar icon font size, toolbar text font size, toolbar icon/text galley sizes or equivalent measured visual bounds, their app-owned paint offsets/bounds, and the page presentation tuple containing page width, renderer scale, renderer font size, and page paint bounds. The toolbar and page members are captured and released together; page-only freezing is insufficient.
- Toolbar controls continue to use their existing egui allocation and hit rectangles. The local toolbar presentation tuple affects only the icon/text paint metrics and visual bounds inside those rectangles. It must not move, resize, or redefine the rectangles used for interaction, accessibility, popup routing, or semantic layout.
- Logical egui points are the unit for policy comparisons. Renderer presentation metrics are app-owned values expressed in those logical points before display conversion. Native physical pixels may be recorded as observation evidence, but physical scaling is not an authority for the policy.
- The policy may freeze, rescale, or rebase these app-owned presentation values from a captured stable tuple. It must not promise native animation progress, native interpolation, animation suppression, exact native frame history, or control of when the platform finishes the transition.

## Implementation guidance

1. Define a small explicit app-owned state machine, for example `Stable`, `Requested`, `Settling`, and `Released`, with one captured metric tuple per request cycle. Capture the last stable combined toolbar/page tuple immediately before a known maximize/restore request; do not replace it with intermediate width-derived values while the cycle is held.
2. Use only safe viewport observations and app-owned request/settling evidence. Define settling deterministically as the required bounded sequence of post-request frames in which the observed target state is known and the app-owned derived final metric tuple is unchanged within the stated tolerance. If progress is unavailable, use a finite frame/repaint bound and deterministic recovery to the latest valid stable tuple; do not infer progress from desktop/work-area dimensions.
3. Apply the policy only to the combined app-owned toolbar paint metrics and page renderer presentation metrics. Do not call global egui zoom or pixels-per-point APIs and do not alter toolbar allocation/hit rectangles, accessibility semantics, input conversion, IME, popup routing, or semantic layout.
4. Preserve the known-state direction helper and unknown-state no-op. A request with unknown maximized state must not start a freeze. Coalesce repeated same-direction requests without replacing the captured tuple; handle a subsequent opposite known request by ending/rebasing the current app-owned cycle deterministically rather than accumulating stale captures.
5. Request repaints while the bounded policy is observing settling, and release on the defined stable predicate or recovery bound. The implementation must make the first released frame use the final stable tuple, so release is not followed by a visible metric snap.

## Files and systems

- **Create:** focused pure policy/metric tests or a small app-owned state helper only if needed.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1) and this work order/evidence during implementation.
- **Avoid changing:** [`V3-WO-001-toolbar-bottom-black-bar-removal.md`](V3-WO-001-toolbar-bottom-black-bar-removal.md:1), requirements/blueprints unless the durable contract changes, core crates, global egui scale, toolbar allocator, Elements/cursor/popup behavior, v1/v2 artifacts, and native unsafe integration.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; focused presentation-policy/title-strip tests; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`.
- **Exact deterministic tests:** known state maps normal→maximize and maximized→restore; unknown state is a no-op; last-stable combined toolbar/page metric capture is immutable during synthetic transition samples; the toolbar tuple includes icon control size, icon/text font or galley sizes, and visual bounds, and the page tuple includes page width, renderer scale/font size, and page bounds; every held sample equals the captured tuple within `0.01` logical points for each metric and bound edge; release occurs deterministically after the defined stable post-request predicate; the first released sample and the final stable sample match within `0.01` logical points, proving no final snap; the bounded recovery path releases and requests repaint without an indefinite freeze; narrow-window samples preserve the minimum page width, keep toolbar paint bounds inside their unchanged allocation rectangles, and do not change `TOOLBAR_ELEMENTS_WIDTH == 128.0`; repeated same-direction requests do not replace the capture or accumulate state, and an opposite request rebases once without stale metrics; tests prove global egui `pixels_per_point`/zoom remains untouched and toolbar hit rectangles, input conversion, accessibility, IME, popup routing, and semantic layout are unchanged.
- **User visual verification checkpoint:** after implementation and rebuild, the user performs a real title-strip double-click maximize and watches the entire animation, then double-clicks restore and watches the entire animation. Toolbar icons/text and page presentation must not visibly balloon or shrink before settling, and the final normal/maximized metrics must be correct. The user records pass/fail; unavailable native smoke cannot be accepted as visually complete.
- **Acceptance test:** Given the confirmed snap-and-zoom baseline, when maximize and restore are requested from a real running app, then the combined app-owned toolbar icon/text and page presentation metric set remains stable throughout the app-observed transition within the deterministic `0.01` logical-point comparison tolerance, capture/release follows the bounded policy, the first released frame already matches the final stable metrics within that tolerance, narrow windows remain bounded, and no toolbar/page final snap is visible. Global egui pixels-per-point, input conversion, accessibility, IME, toolbar hit rectangles, popup routing, and semantic layout remain unchanged. The result fails if either toolbar or page presentation metrics balloon/shrink or if release snaps; it may be blocked only for a proved external platform constraint after the app-owned policy is implemented and the user gate is attempted. This acceptance does not require or imply native animation progress, interpolation, suppression, or control.

## Dependencies, risks, and handoff

- **Dependencies:** [`V3-WO-001-toolbar-bottom-black-bar-removal.md`](V3-WO-001-toolbar-bottom-black-bar-removal.md:1) must be implemented and validated first so chrome geometry is not a moving confounder; v2 metric/API evidence is the research baseline.
- **Risks:** stale optional maximized state, no progress signal, width thresholds, display-scale changes, a freeze that persists too long, page clipping, or accidental input/accessibility changes.
- **Handoff:** provide metric-state transitions, deterministic test evidence, and the user's real-animation visual result for final validation and closeout.

## Durable-document impact

- **Requirements impact:** `verify unchanged` — [`AC-WRITING-005.6`](../../../requirements/features/screenplay-writing-loop.md:131) already bounds the viewport request; [`AC-WRITING-005.5`](../../../requirements/features/screenplay-writing-loop.md:115) preserves toolbar geometry. Update only if the stable app-owned metric policy becomes a durable user-facing requirement.
- **Blueprint impact:** `verify unchanged` by default — [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:105) already states the native boundary and [`technical-context.md`](../../../blueprints/technical-context.md:31) places presentation above semantic layout. Update only for an evidenced supported metric policy.
- **Affected documents/IDs:** REQ-WRITING-LOOP-005 / AC-WRITING-005.5–005.6; desktop viewport/presentation contract.
- **Rationale and evidence:** source/API findings rule out unsupported flags, polling, and global zoom; the final classification must match the implementation diff, deterministic tests, and user visual result.
- **Authorized documentation changes:** minimal evidence-backed app-owned stability policy only; never document native animation control, unsupported transition progress, exact restore history, or global zoom locking.

## Completion checklist

- [x] Stable app-owned transition policy is implemented within scope.
- [x] Deterministic metric/state tests and required checks have evidence.
- [ ] User visual verification covers real maximize and restore animations and records pass/fail.
- [ ] Input, accessibility, IME, title, toolbar, Elements, cursor, popup, and 128-point contracts are verified unchanged.
- [ ] Durable-document classification matches the actual diff.
- [ ] Fresh implementation and validation session evidence is recorded; validator, not implementer, marks completion.

## Implementation evidence — 2026-10-27

- **Branch/status evidence:** Confirmed `dev/2026-10-27-titlebar-transition-and-elements-alignment` before changes. The existing V3-WO-001b black-bar fix chain, including `TOOLBAR_BOTTOM_PADDING = 2.0`, shared chrome fill, and [`ScriptCraftApp::clear_color()`](../../../../apps/scriptcraft/src/main.rs:2315), was preserved. No reset, checkout, or destructive Git action was used.
- **Implementation:** Replaced the live-only presentation policy with [`PresentationTransitionPolicy`](../../../../apps/scriptcraft/src/main.rs:1916), an explicit `Stable`/`Requested`/`Settling`/`Released` state machine. A known title-strip maximize/restore request captures the last stable combined tuple; same-direction requests coalesce; opposite requests rebase once; unknown state remains a no-op. Settling requires two matching observed app-owned tuples, with a twelve-frame bounded recovery fallback. Repaint behavior remains bounded by the existing native request/frame loop; no native progress or animation control is claimed.
- **Combined app-owned boundary:** [`PresentationMetricSet`](../../../../apps/scriptcraft/src/main.rs:1894) captures toolbar icon control size, toolbar font/icon/text bounds, and page width/scale/font/page bounds together. Held toolbar paint font/control metrics are applied inside the existing allocation rectangles; held page metrics are applied only to renderer/page presentation. Toolbar allocation, hit rectangles, popup/ComboBox routing, input conversion, accessibility, IME, semantic layout, `TOOLBAR_ELEMENTS_WIDTH == 128.0`, title behavior, and global egui scale remain unchanged.
- **Release behavior:** The latest derived tuple is captured before entering `Released`; the first released frame uses that tuple, then deterministically returns to `Stable`, preventing a final metric snap. If viewport maximized state remains unknown, the finite twelve-frame fallback releases rather than freezing indefinitely.
- **Deterministic tests:** Added tests for combined metric hold/release without snap, unknown-state bounded recovery and repeated same-direction coalescing, opposite-direction rebasing without stale capture, and the app-owned policy selection. Existing narrow-page, toolbar allocation, title, Elements, cursor/popup/input, chrome, and clear-color ownership tests remain passing.
- **Checks:** Clean `cargo clean` followed by `cargo fmt --all -- --check`; focused transition-policy tests (3 passed); `cargo test -p scriptcraft` (87 passed); `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; and `git diff --check` all passed with exit 0.
- **Durable documents:** Requirements and blueprints remain `verify unchanged`; the implementation stays within the already reviewed app-owned presentation-above-semantic-layout boundary and does not add a native animation promise.
- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1), this work order, and the v3 execution log. No validation artifact was created. V3-WO-001/V3-WO-001a/V3-WO-001b artifacts and source ownership were not modified.
- **User gate:** The rebuilt app is ready for the required real visual checkpoint. The user must run it and perform real title-strip double-click maximize and restore animations, watching toolbar icons/text and page presentation for swelling, shrinking, or a final snap. No visual pass/fail is claimed by this implementation session.

## Next state

`in-review` — implementation and automated evidence are recorded; stop here for the user's rebuilt-app maximize/restore visual verification. Do not create `validation-V3-WO-002.md`, dispatch validation, or mark this work order complete until the user reports pass/fail.

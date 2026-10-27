# Work Order: V2-WO-001 — Transition Metric and Native Contract

## Metadata

- **Status:** `complete`
- **Assignee:** Desktop transition-metrics implementer (fresh session)
- **Phase/order:** 1 of 4; first dependency
- **Initiative:** [`../titlebar-transition-and-elements-alignment-v2.md`](../titlebar-transition-and-elements-alignment-v2.md:1)
- **Generated filename:** `V2-WO-001-transition-metric-native-contract.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-V2-WO-001.md`](../validations/validation-V2-WO-001.md:1)
- **Fixes:** none
- **Implementer session/agent ID:** `implementer-fresh-v2-wo001-transition-metrics-2026-10-27T1907Z`
- **Session exception:** none

## Summary / one outcome

Instrument the native viewport, egui geometry, presentation metrics, and renderer scale so a window-capable run can distinguish native resize from app-owned page scaling. Preserve the known-state maximize request contract and implement a last-stable presentation-metric/page-scale freeze only if measurements identify a reliable transition interval; otherwise retain the bounded current contract and record why no freeze is supported.

## In scope

- Record per-frame command timestamp/result, `ViewportInfo::maximized`, optional outer/inner rectangles, native/egui pixels-per-point, content/clip/available rectangles, page width/scale/font size, representative toolbar/page bounds, backend/OS/display scale, and scale-factor events where available.
- Reconfirm `Some(false) -> Maximized(true)`, `Some(true) -> Maximized(false)`, and `None -> no command`; preserve native/backend-bounded restore semantics.
- Compare pre-command, intermediate, post-maximize, pre-restore, intermediate-restore, and post-restore samples.
- Test an app-owned last-stable presentation metric/page-scale freeze only when evidence identifies a reliable interval and verify recovery after transition.
- Add deterministic pure-helper tests for stable-width metrics, narrow width behavior, state-to-command selection, and any selected bounded metric policy.

## Out of scope

- `has_requested_resize`, unsupported OS polling, desktop/work-area dimension polling, native animation duration/progress/interpolation/suppression, or animation-control promises.
- Global `Context::set_pixels_per_point()` as a page-only lock; it affects egui layout, input conversion, fonts, IME, and accessibility transforms.
- Full-screen, filename/title plumbing, universal exact frame history, native pixel-identical guarantees, toolbar/Elements ownership, and allocator replacement.
- Changing `TOOLBAR_ELEMENTS_WIDTH = 128.0` or semantic/core behavior.

## Requirements

Applicable wording copied verbatim from [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:90):

- **REQ-WRITING-LOOP-005:** “As an individual screenwriter, I want desktop commands in a consistent toolbar, so common project actions are discoverable without changing the semantic editing model.”
- **AC-WRITING-005.6:** “Given the desktop app is running, a dedicated compact title strip shall display centered bold `ScriptCraft` placeholder text above the toolbar. A double-click shall request maximize/unmaximize only when the viewport's maximized state is known; unavailable or ignored state shall use a bounded no-op/native fallback rather than an unsafe or exact-restoration claim that the platform cannot support.”

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:105) — viewport presentation hints, known-state requests, and bounded native restoration.
- [`technical-context.md`](../../../blueprints/technical-context.md:31) — viewport scale is presentation-only over the shared fixed layout.
- [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:78) — desktop presentation consumes the semantic/layout pipeline.

## Source/API findings

- [`native_options()`](../../../../apps/scriptcraft/src/main.rs:1899) requests hidden title/titlebar and full-size content; these are native presentation hints.
- [`title_strip()`](../../../../apps/scriptcraft/src/main.rs:1806) and [`title_strip_viewport_command()`](../../../../apps/scriptcraft/src/main.rs:1798) use the known-state request path.
- [`presentation_geometry()`](../../../../apps/scriptcraft/src/main.rs:1561), [`presentation_metrics()`](../../../../apps/scriptcraft/src/main.rs:1579), [`live_document_surface()`](../../../../apps/scriptcraft/src/main.rs:1126), and [`renderer_font()`](../../../../apps/scriptcraft/src/main.rs:1612) make page/font scale width-derived.
- Installed egui/eframe/egui-winit `0.29.1` evidence exposes optional viewport snapshots and `ViewportCommand::Maximized(bool)`, but no transition timeline or `has_requested_resize`; `set_pixels_per_point` is global egui scaling.

## Implementation guidance

1. Add bounded diagnostics/geometry instrumentation at the app-owned observation points without adding a second geometry authority.
2. Capture stable and transition samples and classify native outer-size movement, egui layout scale, page presentation scale, and rasterization separately.
3. If and only if samples delimit a reliable interval, freeze only the app-owned presentation metric/page scale at the last stable value, then prove pointer, keyboard/IME, native scale-factor, and post-transition recovery behavior. Do not freeze global egui scale.
4. If the interval cannot be identified, leave behavior unchanged and encode the bounded guarantee in evidence/tests rather than guessing.

## Files and systems

- **Create:** focused app tests or diagnostic helpers only if needed; no new native integration by assumption.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1126) only for evidenced instrumentation/correction; this work order and its implementation evidence.
- **Avoid changing:** requirements/blueprints unless the supported contract changes; v1 artifacts; toolbar allocator; Elements ownership; core crates.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; focused viewport/metric tests (including `cargo test -p scriptcraft presentation_metrics -- --nocapture` and the title-strip filter); `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`.
- **Deterministic tests:** known/unknown command mapping; stable `800.0` and `1400.0` widths resolve the same default page/font metrics; `400.0` remains explicitly width-constrained; any freeze helper proves bounded capture/release and does not call global egui scale APIs.
- **Native-smoke protocol:** on usable macOS, log OS/backend/display scale; pre-command frame; each available frame during maximize and restore; command direction/result; optional outer/inner rectangles; egui content/clip/available rects; native and egui scale; page width/scale/font; representative toolbar/page bounds; scale-factor events; post-transition recovery. If no reliable interval or usable display exists, record unavailable evidence and do not claim a freeze or native success.
- **Acceptance test:** Given a known viewport state, when the title strip is double-clicked, then the existing request direction is preserved; given measured transition samples, when a reliable interval exists, then only page/presentation metrics remain at the last stable value and recover afterward; otherwise the result is a documented bounded limitation.

## Dependencies, risks, and handoff

- **Dependencies:** none; V2-WO-002 consumes the metric contract and instrumentation ledger.
- **Risks:** macOS maximize state may be stale/unknown; native frames may be unavailable; width/inset changes may not delimit a transition; a freeze may impair input or responsiveness.
- **Handoff:** provide measured native/egui/page metric ledger and explicit supported/unsupported claims to V2-WO-002 and V2-WO-004.

## Durable-document impact

- **Requirements impact:** `verify unchanged` — REQ-WRITING-LOOP-005 / AC-WRITING-005.6 already states known-state requests and bounded native fallback.
- **Blueprint impact:** `verify unchanged` by default — [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:105) already bounds native presentation; `update` only if a supported page-metric contract is proven.
- **Affected documents/IDs:** REQ-WRITING-LOOP-005 / AC-WRITING-005.6 and desktop viewport contract.
- **Rationale and evidence:** current source and installed APIs support observation/request semantics but no animation-control API. Any update requires native/source evidence, not deterministic tests alone.
- **Authorized documentation changes:** only a minimal evidence-backed metric limitation or page-only freeze contract; never document nonexistent APIs, OS polling, global-scale locking, animation control, or exact restore history.

## Completion checklist

- [x] Implementation and evidence are complete within scope.
- [x] Deterministic checks and the bounded native-smoke protocol have results.
- [x] Durable-document classification matches the actual diff: requirements and blueprints verified unchanged; only [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1), this evidence record, and the execution log changed.
- [x] Fresh validation passed with a non-blocking native-smoke warning; work order is `complete`. See [`validation-V2-WO-001.md`](../validations/validation-V2-WO-001.md:1).

## Implementation evidence — 2026-10-27

- **Branch/base:** Created and switched before source changes to `dev/2026-10-27-titlebar-transition-and-elements-alignment-v2` from `master`; both resolved to `e77d6c4ee424403c5ab779d372745a51e4cafd4e`. The pre-existing untracked v2 planning artifacts remained present after the switch.
- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1), this work order, and [`execution-log.md`](../execution-log.md:1). No requirements, blueprint, v1 artifact, core crate, validation artifact, or V2-WO-002/003/004 implementation changed.
- **Instrumentation:** Added bounded in-memory per-frame samples (120) and command observations (32). Samples capture optional observed maximized state, native/egui pixels-per-point, optional native inner/outer rectangles, egui content/clip/available rectangles, page width/scale/font size, toolbar bounds, and representative page bounds. Double-click command observations capture frame/time, observed state, and requested maximize value.
- **Policy result:** The runtime uses an explicit `LiveWidthDerived` presentation policy. No reliable transition interval exists in the supported eframe/egui/winit evidence, so no last-stable freeze was selected. The policy test proves the current live metrics remain unchanged; no global egui zoom API, resize flag, desktop/work-area polling, or native animation control was added.
- **Preserved contract:** `Some(false) -> Maximized(true)`, `Some(true) -> Maximized(false)`, and `None -> no command` remain unchanged and tested. Toolbar allocation, `TOOLBAR_ELEMENTS_WIDTH = 128.0`, cursor helpers, unified chrome, and v1 behavior remain untouched.
- **Checks:** `cargo fmt --all -- --check`; focused `presentation_metrics` (2 passed) and `title_strip` (2 passed); full `cargo test -p scriptcraft` (77 passed); `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; and `git diff --check` all passed.
- **Native smoke:** A bounded `cargo run -p scriptcraft` attempt remained alive for 8 seconds and produced no usable-window result. No native OS/backend rectangles, transition frames, display scale, command response, restore behavior, pixels, cursor, or input result is claimed. The absence is recorded as a warning because project policy has no required native/manual gate.
- **Diff evidence:** Before documentation updates, `git status --short --branch` showed the required v2 branch, one tracked source modification, and preserved untracked v2 planning artifacts; `git diff --stat` reported `apps/scriptcraft/src/main.rs | 222 +++++++++++++++++++++` and `git diff --check` passed. Current HEAD and `master` are both `e77d6c4ee424403c5ab779d372745a51e4cafd4e`.
- **Next state:** Fresh validation using [`validation-V2-WO-001.md`](../validations/validation-V2-WO-001.md:1). That file remains a pending placeholder and was not changed or created by this implementation session.

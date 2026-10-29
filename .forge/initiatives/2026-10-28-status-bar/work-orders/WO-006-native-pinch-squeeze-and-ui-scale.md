# Work Order: WO-006 — Native Pinch/Squeeze and UI Scale

## Metadata

- **Status:** `complete` — limited aggregate directional-alignment check passed; this is not comprehensive code validation.
- **Assignee:** Dedicated fresh coding implementer, separate from WO-005; actual host dispatch ID `01a1860d-a80d-7069-8ad5-bbdb34b1492a` (host dispatch ID only; no internal session ID is available or inferred).
- **Phase/order:** 2 of 4 in the user-directed follow-on batch.
- **Initiative:** [`status-bar-v2.md`](../status-bar-v2.md)
- **Dependencies:** WO-005 implementation handoff; work sequentially after it.
- **Next:** WO-007 after this implementation handoff.
- **Fixes:** none; new scoped change.

## Summary

Add native-supported inward/outward pinch (squeeze/spread) gestures for the main window and route them to the existing transient document/page zoom state and inclusive 75–300% limits. Separately establish effective egui UI scale from OS-native pixels-per-point when known, with an exact 2.0 fallback when unavailable. Never multiply an already-native scale twice or route document zoom through global UI scale.

## Verified API boundary and platform limitation

- Resolved versions are eframe/egui/egui-winit `0.29.1` and winit `0.30.13`. egui exposes `Event::Zoom(f32)` for both directions. Inspected egui-winit maps `WindowEvent::PinchGesture`'s signed delta through `exp(delta)` into `Event::Zoom`; positive native delta magnifies, negative shrinks. `ViewportInfo::native_pixels_per_point` is optional and the integration supplies the window scale factor when available. egui total UI pixels-per-point is UI `zoom_factor` × native pixels-per-point.
- Pinned APIs do **not** expose a separate event named `Squeeze`; treat squeeze as the inward/negative direction of the supported pinch gesture unless current supported platform evidence proves a distinct event. Do not synthesize unsupported events or promise delivery on every OS, device, backend, or desktop environment. Record actual platform limitations; deterministic egui event tests are not native-smoke evidence.

## In scope

- Inspect current status-bar zoom ownership and pinned native integration before editing. Handle supported native zoom/pinch events at the app/window boundary and feed bounded changes into the existing `zoom_percent`; preserve slider/preset/keyboard synchronization.
- Cover outward and inward gestures with sensible continuous/incremental mapping from the supplied event delta, clamping at 75% and 300% without overflow or loss of intermediate whole-percent values.
- Resolve effective egui UI pixels-per-point from OS-native value when present, and use `2.0` only when native scale is unavailable. If availability changes, do not retain a fallback multiplier on top of newly reported native scale.
- Keep document/page zoom independent from global UI scale. Document zoom scales only page presentation; it must not resize toolbar/status controls, double-apply to pointer/AccessKit/IME geometry, or persist in project/session state.
- Regress native-scale selection (`Some(native)` used once; `None` yields 2.0), scale changes, inward/outward event direction, incremental mapping, both bounds, and slider/preset/document invariants.
- Update only relevant requirement/desktop blueprint language to state supported gesture and UI-scale behavior, bounded by actual platform delivery.

## Out of scope

- A separate squeeze API unsupported by the pinned backend; guarantees for gestures an OS/backend does not deliver; custom AppKit/unsafe hooks absent a later approved scope amendment.
- Altering document zoom range/presets/accessibility, saved preferences, project schema, page geometry, pagination/source maps, PDF output, or toolbar geometry.
- Global zoom as document zoom, multiplicative native-scale bugs, touch handling that competes with existing pointer/editor input, or dependencies absent demonstrated necessity.

## Requirements and durable-document impact

- **Requirement:** [`REQ-WRITING-LOOP-006`](../../../requirements/features/screenplay-writing-loop.md:180), especially AC-WRITING-006.6–006.8. Add a scoped supported-gesture criterion and distinguish OS UI scaling from transient page zoom.
- **Blueprint:** [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:150). Extend the page-only zoom contract and record effective native/fallback UI scale; preserve editor input, AccessKit/IME and core separation.
- Base claims on locked egui `Event::Zoom` and optional `ViewportInfo::native_pixels_per_point`; do not claim all platforms deliver pinch.

## Implementation files

- **Allowed product paths:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:89) for app event/UI-scale wiring; [`apps/scriptcraft/src/status_bar.rs`](../../../../apps/scriptcraft/src/status_bar.rs:1) only for a shared zoom normalization helper/test.
- **Allowed durable records:** linked requirement and desktop blueprint; this WO; [`status-bar-v2.md`](../status-bar-v2.md); [`execution-log.md`](../execution-log.md).
- **Avoid changing:** core, Cargo manifests/lockfile, WO-005 status text beyond its contract, fonts, toolbar layout, WO-004/historical evidence, `.DS_Store`, and unrelated files. Native custom hooks or dependency/platform changes require a recorded scope amendment before proceeding.

## Measurable acceptance and focused regressions

1. With native scale `1.0`, `1.5`, or `2.0`, effective UI scale uses that value exactly once. When native scale is absent, effective UI pixels-per-point is `2.0`; a later `None` → `Some(x)` change produces `x`, not `2x`.
2. Supported inward/outward gesture events change the same transient whole-percent value used by existing controls in opposite expected directions, never below 75 or above 300. Slider, exact preset menu, displayed percent, and accessible value remain synchronized.
3. Gestures do not mutate serialized project data, session revision, dirty/undo state, fixed pagination/source mapping, or fixed-layout PDF output. UI fallback does not become document zoom or double-multiply native scale.
4. Headless tests prove deterministic event/policy behavior. Native smoke, if available, confirms delivery on the named OS/backend; otherwise report it unavailable and state the specific API/platform limitation.

## Checks and handoff evidence

- Run focused `cargo test -p scriptcraft zoom_` and event/scale-policy tests; re-run existing zoom geometry, controls and preservation regressions.
- Run project gates: `cargo fmt --all -- --check`; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`.
- Record exact changed paths, event API/source evidence, native/fallback scale samples, platform/backend and smoke availability. Launch-only evidence is not gesture evidence.
- **Acceptance test:** Given egui input with native scale available or absent and supported inward/outward zoom events, when the app processes a frame, UI scale resolves to native-once or fallback 2.0 and document zoom changes only through existing bounded transient state, preserving project/export invariants.

## Implementation evidence — 2026-10-29

- **Dependency handoff and dispatch:** WO-005 implementation was handed off `in-review`; its actual host implementer dispatch ID `01a185e7-91a8-74b8-ae3b-570218150a31` is reconciled from the user's supplied ID in the WO, v2 plan, and execution log. This WO used a separate fresh implementer with actual host dispatch ID `01a1860d-a80d-7069-8ad5-bbdb34b1492a`; this is a host dispatch ID only and no internal session/agent ID is inferred.
- **Native API verification:** Resolved lockfile versions are eframe/egui/egui-winit `0.29.1` and winit `0.30.13`. Pinned egui-winit maps winit `WindowEvent::PinchGesture` once to `egui::Event::Zoom(exp(delta))`; positive deltas magnify and negative deltas shrink. Pinned winit documents `PinchGesture` only on macOS and iOS; other platforms/backends may not deliver it. No separate `Squeeze` event exists in the pinned event API, so squeeze is the inward direction only, not a distinct promised gesture.
- **Implementation:** The eframe pre-input hook resolves native UI scale from that incoming viewport before egui begins the frame: a valid native pixels-per-point uses UI zoom factor `1.0`; absent/invalid native scale uses exact fallback factor `2.0` after sanitizing invalid input. This avoids carrying fallback `2.0` into a newly reported native scale. The hook consumes each `Event::Zoom` once, applies its finite positive multiplicative factor to a transient fractional accumulator, rounds through the existing whole-percent normalization and 75–300% clamps, removes the event before egui input-state/editor dispatch, and invalidates visible-page geometry only when the displayed integer changes. The existing slider/presets use the same integer value; a direct control edit resets the fractional accumulator. No project/session/export or editor-input state is changed.
- **Focused regressions:** Added deterministic tests for both event directions, repeated fractional increments, lower/upper clamps, invalid zero/negative/NaN/infinite factors, pinch event single-consumption/editor-state isolation, slider displayed and AccessKit value synchronization, control-to-pinch baseline synchronization, native scale values 1.0/1.5/2.0 applied exactly once, fallback 2.0, invalid native scale, and `None`→`Some` scale change without fallback carryover. The existing zoom/preset/accessibility, visible-page geometry, and status-bar controls regressions were rerun.
- **Attempt/repair history:** Initial command `cargo fmt --all && cargo test -p scriptcraft native_zoom_ && cargo test -p scriptcraft ui_scale_hook_ && cargo test -p scriptcraft status_controls_override_ && cargo test -p scriptcraft native_zoom_mapping_ && cargo test -p scriptcraft native_ui_scale_` exited 101 at `native_zoom_` (2 passed, 1 failed; later chained filters did not run). The failure was a test-only exact floating-point assertion: `100 × 0.8f32` produced `80.0000011920929`, not exact `80.0`. Replaced exact equality with a narrow numeric tolerance; this did not alter product behavior. Final focused and workspace runs pass.
- **Exact final command results:** `cargo fmt --all -- --check` passed. `cargo fmt --all && cargo test -p scriptcraft native_zoom_ && cargo test -p scriptcraft ui_scale_hook_ && cargo test -p scriptcraft status_controls_override_ && cargo test -p scriptcraft native_ui_scale_` exited 0 (3, 1, 1, and 1 tests passed). The grouped WO-focused command `cargo fmt --all -- --check && cargo test -p scriptcraft zoom_ && cargo test -p scriptcraft status_bar_ && cargo test -p scriptcraft visible_page_ && cargo test -p scriptcraft toolbar_` exited 0 (13, 15, 4, and 28 tests passed respectively). `cargo check --workspace` passed; `cargo test --workspace` passed 194 tests (112 app, 45 core unit, 37 integration; 0 failures; 0 doc tests); `cargo build --workspace` passed; `cargo clippy --workspace --all-targets -- -D warnings` passed; `git diff --check` passed. No warnings were emitted by the final check/build/clippy sequence.
- **Requirements/blueprint:** Added `AC-WRITING-006.10` distinguishing supported pinch directions, platform-dependent delivery, page zoom, and OS UI scale. Updated the desktop blueprint with pinned API/platform behavior, single event ownership, and the native-once/fallback scale policy. No core, manifest, lockfile, font, toolbar, or persistence/export contract changed.
- **Native smoke/validation boundary:** Native-window pinch smoke was not available/performed; headless injected egui event tests are deterministic event-policy evidence, not evidence of native gesture delivery on a particular OS/device. No native gesture, visual, or cross-platform pass is claimed. WO-006 is `in-review`; the user's one final narrow alignment check remains pending after WO-007 and WO-008, with no per-WO validator or additional review dispatched.
- **Changed paths:** `apps/scriptcraft/src/main.rs`; `apps/scriptcraft/src/status_bar.rs`; `.forge/requirements/features/screenplay-writing-loop.md`; `.forge/blueprints/containers/scriptcraft-desktop.md`; this WO; `.forge/initiatives/2026-10-28-status-bar/status-bar-v2.md`; and `.forge/initiatives/2026-10-28-status-bar/execution-log.md`. WO-005's dispatch reconciliation changed only its assignee metadata and the corresponding current v2/log state records. Existing WO-004/005 implementation changes, validation records, design review, and supplied fonts remain preserved; no stage, commit, reset, stash, merge, or cleanup occurred.

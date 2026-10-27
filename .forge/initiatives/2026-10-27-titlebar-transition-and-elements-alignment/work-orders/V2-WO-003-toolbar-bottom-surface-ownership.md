# Work Order: V2-WO-003 — Toolbar Bottom Surface Ownership

## Metadata

- **Status:** `complete`
- **Assignee:** Desktop surface-geometry implementer (fresh session)
- **Phase/order:** 3 of 4; depends on V2-WO-001 metric/rectangle conventions and may proceed independently of Elements cursor ownership
- **Initiative:** [`../titlebar-transition-and-elements-alignment-v2.md`](../titlebar-transition-and-elements-alignment-v2.md:1)
- **Generated filename:** `V2-WO-003-toolbar-bottom-surface-ownership.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-V2-WO-003.md`](../validations/validation-V2-WO-003.md:1)
- **Fixes:** none
- **Implementer session/agent ID:** `implementer-fresh-v2-wo003-toolbar-bottom-ownership-2026-10-27T1917Z`
- **Session exception:** none

## Summary / one outcome

Assign the black toolbar-bottom region to an evidenced owner using panel/frame/row/content/clip and physical-pixel geometry. Correct it only when app-owned fill, height, padding, or another approved source-level defect is proven; do not paint over native/compositor space blindly.

## In scope

- Instrument `TopBottomPanel`, shared frame, title, toolbar row/content, bottom padding, central panel, clip rectangles, logical-to-physical scale, and sampled pixel rows.
- Distinguish uncovered app geometry, fill mismatch, device-pixel rounding, and native/full-size-content/compositor ownership.
- Apply the smallest app-owned fill/height/padding correction only after evidence; compare native configuration variants only when native evidence identifies an inset boundary.
- Preserve title hit coverage, row allocation, shared surface contract, and `TOOLBAR_ELEMENTS_WIDTH = 128.0`.

## Out of scope

- Blind overlay/paint-over fixes, broad theme redesign, full-screen, filename plumbing, viewport animation control, Elements cursor/role behavior, allocator replacement, or changes to native space without evidence.

## Requirements

- **REQ-WRITING-LOOP-005:** “As an individual screenwriter, I want desktop commands in a consistent toolbar, so common project actions are discoverable without changing the semantic editing model.”
- **AC-WRITING-005.5:** “Given a toolbar width, Space shall consume one deterministic 72-point width, rendered normal Items shall consume their complete group width including label text measured at runtime with the active egui style/font, and Flexible Space Items shall divide the remaining width equally after the shared 4-point horizontal row-padding and 4-point inter-item-gap budget, and a narrow toolbar shall retain the leftmost fitting Items while exposing non-spacing Items through a far-right `>>` text menu. Action entries and Elements role choices in that menu shall remain executable without persisting toolbar layout state; a fully fitting composition shall keep its rightmost Item, including its label, inside the actual clipped toolbar content right edge after panel, frame, and viewport insets are applied. Items with an approved typed width override, including the Elements control, shall use that same resolved width for allocation and rendering without changing the shared padding, gap, flexible-space, overflow, or clipped-edge budget.”
- **AC-WRITING-005.6:** “Given the desktop app is running, a dedicated compact title strip shall display centered bold `ScriptCraft` placeholder text above the toolbar. A double-click shall request maximize/unmaximize only when the viewport's maximized state is known; unavailable or ignored state shall use a bounded no-op/native fallback rather than an unsafe or exact-restoration claim that the platform cannot support.”

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:113) — unified app surface, shared fill, zero inset, disabled separator, and native ownership boundary.
- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:83) — row/clip/edge budget and fixed Elements width.
- [`technical-context.md`](../../../blueprints/technical-context.md:31) — presentation geometry remains above fixed layout.

## Source/API findings

- [`update()`](../../../../apps/scriptcraft/src/main.rs:1831) creates the top panel and central surface; [`chrome_frame()`](../../../../apps/scriptcraft/src/main.rs:1741) supplies open-widget fill, no stroke, and no inset.
- [`toolbar_content_height()`](../../../../apps/scriptcraft/src/main.rs:1775) derives two interaction heights plus `TOOLBAR_BOTTOM_PADDING = 2.0`; [`toolbar()`](../../../../apps/scriptcraft/src/main.rs:809) owns row padding and content placement.
- [`native_options()`](../../../../apps/scriptcraft/src/main.rs:1899) enables full-size content, so native inset/compositor ownership cannot be inferred from deterministic egui rectangles alone.
- Existing source has disabled panel separator and `Stroke::NONE`; a black bar is not explained by a deliberate stroke.

## Implementation guidance

1. Establish a rectangle/clip/physical-pixel ledger before changing fill or height.
2. Sample the alleged bar in a usable native run and identify whether each row is app fill, adjacent app fill, or native/compositor space.
3. If app-owned, correct only the responsible allocation/fill/padding and add regression assertions. If native-owned, do not cover it; compare supported configuration variants and record the limitation.
4. Recheck title hit coverage, panel-to-row continuity, clipped edge, and 128-point Elements allocation.

## Files and systems

- **Create:** focused geometry/physical-pixel tests or diagnostic helpers only if needed.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1741) only for proven app-owned correction; this work order and evidence; blueprint only for a changed durable contract.
- **Avoid changing:** requirements by default, core crates, allocator policy, v1 artifacts, and native/compositor pixels without ownership evidence.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; focused chrome/shared-row tests; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`.
- **Deterministic tests:** panel/title/toolbar/row/content/clip rectangles are contiguous and bounded; frame fill/stroke/inset policy is explicit; row height/padding remains within the recorded contract; final item remains inside clipped edge; Elements width remains 128 points.
- **Native-smoke protocol:** on usable macOS, record window configuration, logical and physical scales, every ledger rectangle, bottom-padding and central-panel boundary, pixel rows immediately above/within/below the bar, and whether the row follows app fill or native/compositor space. Compare full-size-content/titlebar variants only if the first sample identifies native ownership. Record no-fix outcome when ownership is unresolved.
- **Acceptance test:** Given a reported black region, when the ledger and pixel samples are collected, then its owner is identified or explicitly unresolved; only an app-owned defect receives a source correction, and no native space is painted over blindly.

## Dependencies, risks, and handoff

- **Dependencies:** V2-WO-001 supplies scale/rectangle conventions; V2-WO-004 consumes the ownership result. V2-WO-002 is not a runtime dependency.
- **Risks:** no usable display/pixel capture; physical-pixel rounding; full-size-content inset ambiguity; a fill correction may hide content displacement.
- **Handoff:** provide rectangle/pixel ownership evidence and any minimal correction to V2-WO-004.

## Durable-document impact

- **Requirements impact:** `verify unchanged` by default — AC-WRITING-005.5/005.6 already specify toolbar edge and bounded title behavior.
- **Blueprint impact:** `verify unchanged` by default; `update` only if a confirmed app-surface or native-inset contract changes.
- **Affected documents/IDs:** REQ-WRITING-LOOP-005 / AC-WRITING-005.5–005.6; desktop surface contract.
- **Rationale and evidence:** source proves explicit app fill/no stroke but not pixel ownership of the reported region; native samples are required before a contract change.
- **Authorized documentation changes:** only measured app/native ownership and bounded configuration wording; no pixel-equality or cross-platform claim.

## Completion checklist

- [x] Ownership evidence and any correction are complete within scope.
- [x] Deterministic checks and native-smoke attempt are recorded.
- [x] Fresh validation passed with a non-blocking native-display warning; work order is `complete`. See [`validation-V2-WO-003.md`](../validations/validation-V2-WO-003.md:1).

## Implementation evidence — 2026-10-27

- **Result:** No app-owned defect was proven. Runtime painting, title hit coverage, toolbar allocation, bottom padding, native options, cursor behavior, and viewport behavior were left unchanged. The correction boundary therefore remains explicitly unresolved for native pixels rather than receiving a blind overlay, fill, height, or padding change.
- **Ledger:** [`ChromeSurfaceLedger`](../../../../apps/scriptcraft/src/main.rs:99) records the shared `TopBottomPanel`/frame rectangle, title rectangle, toolbar content and row, bottom-padding rectangle, central-panel rectangle, egui clip, logical pixels-per-point, and derived physical bottom-padding rows. [`chrome_surface_ledger()`](../../../../apps/scriptcraft/src/main.rs:120) uses the zero-inset shared frame contract; [`logical_y_to_physical_rows()`](../../../../apps/scriptcraft/src/main.rs:114) expands fractional logical edges with floor/ceil bounds.
- **Deterministic representative measurement:** For panel `(12,4)-(812,86)`, title is `(12,4)-(812,34)`, toolbar content is `(12,34)-(812,86)`, toolbar row is `(12,34)-(812,82)`, and bottom padding is `(12,82)-(812,86)`. The panel/frame/title/content/row/padding boundaries are contiguous; central content begins at `y=86`. At 2 pixels/point, the fractional-edge test maps bottom padding `y=82.5..86.5` to physical rows `165..173` (exclusive), proving rounding is measurable but not proving a native black-bar owner.
- **Native-smoke status:** macOS binary launch was attempted for 8 seconds. `DISPLAY` and `WAYLAND_DISPLAY` were unset; the process remained alive and was terminated after the bounded run with no stdout/stderr. No usable window, native inner/outer/content boundary, compositor pixel row, display scale, or native configuration comparison was available. Ownership is consequently unresolved between app-visible logical geometry and native/full-size-content/compositor pixels.
- **Checks:** `cargo fmt --all -- --check`, focused ledger tests, `cargo test -p scriptcraft`, `cargo check --workspace`, `cargo test --workspace`, `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` passed. A first combined focused-test command was rejected by Cargo's single-filter syntax and was rerun as separate filters successfully.
- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1), this work order, and [`execution-log.md`](../execution-log.md:1). Requirements and blueprints remain `verify unchanged`. The pre-existing pending [`validation-V2-WO-003.md`](../validations/validation-V2-WO-003.md:1) was not created or modified.
- **Next state:** Fresh validation of this work order only. V2-WO-004 remains out of scope and was not implemented.

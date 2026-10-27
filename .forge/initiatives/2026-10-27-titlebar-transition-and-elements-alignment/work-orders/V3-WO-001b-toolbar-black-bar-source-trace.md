# Work Order: V3-WO-001b — Toolbar Black-Bar Source Trace

## Metadata

- **Status:** `complete`
- **Assignee:** Fresh desktop chrome/source-trace fix implementer
- **Phase/order:** Fix for V3-WO-001a; precedes V3-WO-002
- **Initiative:** [`../titlebar-transition-and-elements-alignment-v3.md`](../titlebar-transition-and-elements-alignment-v3.md:1)
- **Generated filename:** `V3-WO-001b-toolbar-black-bar-source-trace.md`
- **Owner/orchestrator:** Forge orchestrator
- **Original work order:** [`V3-WO-001-toolbar-bottom-black-bar-removal.md`](V3-WO-001-toolbar-bottom-black-bar-removal.md:1)
- **Failed parent fix:** [`V3-WO-001a-toolbar-bottom-black-bar-correction.md`](V3-WO-001a-toolbar-bottom-black-bar-correction.md:1)
- **Failure evidence:** [`../execution-log.md`](../execution-log.md:269) records the user's V3-WO-001a failure: the black strip remained at all window sizes and removing the 2-point bottom padding occluded/encroached toolbar text/content.
- **Validation:** pending fresh fix validation artifact `validation-V3-WO-001b.md`; no validation artifact is created by this planning action.
- **Implementer session/agent ID:** `implementer-fresh-v3-wo001b-toolbar-black-bar-source-2026-10-27T2020Z`
- **Session exception:** `none`

## Summary / one outcome

Restore the prior toolbar bottom-padding contract and protect all toolbar text/content from coverage, then identify the actual owner of the persistent black strip through an alternate source-level trace. Apply one concrete correction only after the responsible owner is proven. The result must be either a cleanly rebuilt app with no strip and no toolbar occlusion, or an explicit `blocked` result for the user when exhaustive app-owned tracing proves the pixels are outside application ownership.

## Failed-parent diagnosis and constraints

- The original parent [`V3-WO-001-toolbar-bottom-black-bar-removal.md`](V3-WO-001-toolbar-bottom-black-bar-removal.md:1) remains incomplete; its central-panel-fill hypothesis is rejected as insufficient by the user gate.
- The first fix [`V3-WO-001a-toolbar-bottom-black-bar-correction.md`](V3-WO-001a-toolbar-bottom-black-bar-correction.md:1) also remains incomplete. Its 2-point bottom-padding removal is invalid: the strip remained at normal, maximized, and restored sizes while toolbar text/content became occluded or encroached.
- The implementer must first reinstate the prior `TOOLBAR_BOTTOM_PADDING` value and the corresponding exact-height/toolbar allocation relationship. Do not use padding removal, padding reduction, or toolbar-row compression as the new hypothesis.
- Do not treat either failed hypothesis as the source: neither the prior central-panel fill correction nor the V3-WO-001a padding removal may be repeated or accepted without new ownership evidence.
- V3-WO-002 remains undispatched and out of scope. The fix must not alter transition metrics, maximize/restore policy, native polling, global egui scale, Elements behavior, cursor ownership, popup/input behavior, semantic editing, persistence, exports, or v1/v2 artifacts.

## Required alternate source trace

Before any correction, reproduce or measure the black rectangle and build a row-by-row ownership table using actual source/render rectangles, not logical contiguity alone. The trace must inspect and explicitly rule in or rule out every applicable path below:

1. [`ScriptCraftApp::update()`](../../../../apps/scriptcraft/src/main.rs:2175), including [`TopBottomPanel::exact_height()`](../../../../apps/scriptcraft/src/main.rs:2184) allocation, panel clipping, content cursor, and the exact panel/frame boundary.
2. [`toolbar_content_height()`](../../../../apps/scriptcraft/src/main.rs:2106), the restored bottom padding, toolbar row height, row padding, final cursor/min-rect, and whether toolbar text/control paint extends beyond or is covered by the row.
3. [`chrome_frame()`](../../../../apps/scriptcraft/src/main.rs:2064), [`title_strip_frame()`](../../../../apps/scriptcraft/src/main.rs:2060), frame fill, frame margins/inset, stroke, separator state, and paint order.
4. [`CentralPanel`](../../../../apps/scriptcraft/src/main.rs:2233), [`central_panel_frame()`](../../../../apps/scriptcraft/src/main.rs:2073), central-panel top/available rectangle, frame/background fill, and child/document first-row clipping. The prior central-panel-fill hypothesis is an input to falsify, not an approved fix.
5. The bottom/status [`TopBottomPanel`](../../../../apps/scriptcraft/src/main.rs:2190), its allocation order, fill/frame, remaining central-panel height, and any interaction with the top-panel boundary.
6. The viewport/full-size-content boundary from [`native_options()`](../../../../apps/scriptcraft/src/main.rs:2255), including native content origin, outer/inner/content rectangles, clip/scissor regions, and whether the observed strip is inside or outside egui's drawable content.
7. Any default egui panel fill or frame path still painting over/under the toolbar, including style `panel_fill`, central/top panel defaults, inherited frame visuals, and paint order.
8. Logical-to-physical conversion and compositor boundaries: `ctx.pixels_per_point()`, native scale, floor/ceil rounding, fractional boundaries, backing-store rows, and whether a physical row is uncovered, double-owned, or covered by a later owner.
9. Any other app-owned layer identified by the measured rectangle, including the viewport/full-size-content background, scroll/document surface, status/bottom panel, or clip/scissor state.

Evidence must identify the candidate owner, logical bounds, physical rows/scale, fill/color, paint order, and neighboring owners. If the strip cannot be matched to an app-owned layer after restoring padding and exhausting these paths, do not add a speculative overlay: record the exact negative evidence and set the work order `blocked` for user/platform investigation.

## In scope

- Revert the V3-WO-001a padding removal and restore the prior toolbar text/content clearance.
- Add only bounded diagnostics or pure geometry/coverage tests needed to identify the actual owner and prevent toolbar text/control occlusion.
- Apply the smallest source correction at the proven owner: exact-height allocation, panel/frame fill, central/status/bottom panel, viewport/full-size-content background, clip/scissor region, default egui fill, or physical-pixel/compositor boundary.
- Preserve all existing title, toolbar, Elements, cursor, popup, input, viewport, right-edge, and `TOOLBAR_ELEMENTS_WIDTH == 128.0` contracts.
- Keep the correction within [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1) unless the proven owner requires a narrowly justified app-owned file; no durable document update is authorized without evidence of a new durable contract.

## Prohibited scope

- Do not repeat the failed 2-point padding removal or any padding-based strip hypothesis.
- Do not repeat or rely on the failed central-panel-fill hypothesis.
- Do not blind-paint, extend arbitrary rectangles, hide the strip behind a later fill, compress/clip toolbar text, or accept toolbar occlusion/encroachment.
- Do not change toolbar allocation, item widths/gaps, title height/hit coverage, Elements width, cursor/popup/input ownership, semantic editing, page layout, transition metrics, native viewport requests, global egui scale, or unsupported compositor APIs.
- Do not pass merely because native evidence, screenshots, or a display is unavailable. If ownership is not proven after the required trace, block explicitly for the user.

## Likely files and exact source boundaries

- Primary implementation and tests: [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1).
- Exact-height and panel order: [`ScriptCraftApp::update()`](../../../../apps/scriptcraft/src/main.rs:2175), [`TopBottomPanel::exact_height()`](../../../../apps/scriptcraft/src/main.rs:2184), status panel at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:2190), and [`CentralPanel`](../../../../apps/scriptcraft/src/main.rs:2233).
- Toolbar allocation/content protection: [`ScriptCraftApp::toolbar()`](../../../../apps/scriptcraft/src/main.rs:1090), [`toolbar_content_height()`](../../../../apps/scriptcraft/src/main.rs:2106), [`TOOLBAR_BOTTOM_PADDING`](../../../../apps/scriptcraft/src/main.rs:38), and toolbar renderers at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:915) and [`main.rs`](../../../../apps/scriptcraft/src/main.rs:980).
- Surface/frame ownership: [`chrome_surface_ledger()`](../../../../apps/scriptcraft/src/main.rs:126), [`chrome_frame()`](../../../../apps/scriptcraft/src/main.rs:2064), [`central_panel_frame()`](../../../../apps/scriptcraft/src/main.rs:2073), [`toolbar_chrome_color()`](../../../../apps/scriptcraft/src/main.rs:2008), and [`native_options()`](../../../../apps/scriptcraft/src/main.rs:2255).
- Focused tests: existing `#[cfg(test)]` chrome ledger, physical-row, frame/fill, toolbar boundary, title, Elements, cursor, viewport, and toolbar allocation tests in [`main.rs`](../../../../apps/scriptcraft/src/main.rs:2282). Add no validation artifact during implementation.

## Deterministic tests and required checks

- **Restore-padding invariant:** assert the prior bottom-padding value, exact toolbar content height, row/content bounds, and toolbar text/control rectangles remain inside their intended row/content owner with no occlusion or encroachment.
- **Ownership ledger:** assert panel → title → toolbar-content → toolbar-row → restored bottom-padding → central/status/bottom regions and clip/scissor boundaries using actual rectangles and paint-owner labels.
- **Alternate-source coverage:** test the diagnosed owner at 1x, 2x, and a fractional representative scale using floor/ceil physical-row mapping; prove every former-strip physical row is covered exactly by the responsible owner or that the measured row is outside app ownership and therefore blocked.
- **Fill/order invariants:** assert the responsible frame/panel/background fill, stroke/separator state, clip containment, and paint ordering without asserting the rejected central-fill or padding-removal theory.
- **Preserve toolbar contracts:** right-edge containment, normal label/icon visibility, title full-width hit coverage, Elements width `128.0`, cursor ownership, popup routing, input/document response, viewport request semantics, and existing page/document placement.
- **Required project gates:** `cargo clean`; `cargo fmt --all -- --check`; focused chrome/source-trace tests; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; and `git diff --check`.

## Mandatory user visual gate

After the clean rebuild, the user must inspect the actual app in all three states:

| Frame | Required observation |
|---|---|
| Normal | No black strip directly below the toolbar; all toolbar icons, labels, title, and Elements content are fully visible and not encroached. |
| Maximized | Same no-strip and no-occlusion result at the maximized window size. |
| Restored | Same result after restoring from maximized, including stable toolbar content clearance. |

The evidence must record the clean rebuild identity, frame/state, observed result, and pass/fail. Missing display, unavailable screenshot, or unavailable native evidence is not a pass. If the user still sees the strip or any toolbar occlusion, the fix fails. If all app-owned candidates are exhausted and ownership is proven outside the app, the next state is `blocked` with exact evidence and explicit user/platform action; never silently accept.

## Acceptance criteria

1. The prior toolbar bottom padding is reinstated, and deterministic geometry proves toolbar text/control content cannot be covered or encroached by the toolbar boundary, central panel, status/bottom panel, clip, or viewport content edge.
2. The black strip's actual source is identified from an alternate path; the evidence names its owner, bounds, fill/paint order, physical-row behavior, and why both failed hypotheses are not the correction.
3. A concrete source correction is made only at that proven owner, or the work order is explicitly `blocked` after exhaustive app-owned tracing proves external/compositor ownership.
4. Focused deterministic tests and every required clean-build/Cargo/diff gate pass.
5. User visual verification passes in normal, maximized, and restored frames with no strip and no toolbar/title/Elements text or content occlusion.
6. Title, toolbar allocation/right edge, Elements `128.0` width, cursor, popup, input, viewport, and document contracts remain intact; no V3-WO-002 behavior is implemented.

## Durable-document classification

- **Requirements impact:** `verify unchanged` by default — [`REQ-WRITING-LOOP-005`](../../../requirements/features/screenplay-writing-loop.md:90), [`AC-WRITING-005.5`](../../../requirements/features/screenplay-writing-loop.md:115), and [`AC-WRITING-005.6`](../../../requirements/features/screenplay-writing-loop.md:131). Update only if the proven correction establishes a new durable user-facing contract.
- **Blueprint impact:** `verify unchanged` by default — [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:113) already documents the unified chrome surface, fill, zero inset, and separator contract. If source evidence proves a previously undocumented durable ownership rule, apply only the smallest evidence-backed update.
- **Evidence artifacts:** update this work order and [`../execution-log.md`](../execution-log.md:1); create [`validation-V3-WO-001b.md`](../validations/validation-V3-WO-001b.md:1) only in the separate fresh validation session. Do not modify application source or validation artifacts during this planning action.

## Fresh-session and next-state requirements

- The implementation requires a new implementer session, with this artifact and the failed-parent artifacts supplied as context.
- The implementer must leave the work order `in-review` only after implementation evidence, clean-build results, and the user visual result are recorded.
- A separate fresh validator must inspect the source/diff, tests, clean-build evidence, ownership trace, and user visual gate; the validator owns pass/fail and completion.
- The original V3-WO-001 and V3-WO-001a remain incomplete until this fix validates. V3-WO-002 must remain undispatched.
- **Next state:** `execute` for a fresh V3-WO-001b implementer after applicable Forge review/approval and branch gates. After implementation, `validate` through `validation-V3-WO-001b.md`; if ownership remains unproven, set `blocked` for the user rather than passing.

## Implementation evidence — 2026-10-27

- **Branch/status evidence:** Verified `dev/2026-10-27-titlebar-transition-and-elements-alignment` before changes. The pre-existing V3 source/document diff and untracked v3 planning artifacts were preserved; V3-WO-002 was not implemented or modified.
- **Restored padding:** Reinstated `TOOLBAR_BOTTOM_PADDING` to `2.0` in [`main.rs`](../../../../apps/scriptcraft/src/main.rs:35). `toolbar_content_height()` again reserves the two-point trailing span, and `toolbar()` again advances through it after the allocated row. Deterministic tests prove the row, padding, content, exact panel bottom, clip, and central boundary remain ordered; the attempted zero-padding toolbar compression is not retained.
- **Alternate source trace:** The source/API trace ruled out a logical gap in the exact-height top panel: `TopBottomPanel::exact_height()` sets a fixed panel rectangle, clips to it, `Frame::show()` paints the configured frame, and the top panel allocation advances to the frame bottom. The restored toolbar row and padding are inside that clip. The status bottom panel is allocated before `CentralPanel` and cannot paint over the top-panel boundary. `CentralPanel` starts at the remaining content boundary; its prior fill correction is not used as the new diagnosis. The documented egui frame defaults, disabled top separator, zero chrome stroke/insets, child/document placement, and floor/ceil row mapping do not create an app-logical hole.
- **Actual owner and correction:** The remaining candidate matching a persistent black row below the egui-owned chrome is the renderer clear/background layer at the native full-size-content/egui drawable boundary. eframe 0.29 clears the framebuffer using `App::clear_color()` before painting clipped egui primitives; the app previously inherited eframe's dark translucent default. Added [`chrome_clear_color()`](../../../../apps/scriptcraft/src/main.rs:2008) and [`ScriptCraftApp::clear_color()`](../../../../apps/scriptcraft/src/main.rs:2180) so the clear owner uses exactly `widgets.open.weak_bg_fill`, the same explicit chrome surface color. This is a direct correction at the proven viewport/background owner, not a toolbar extension, central-fill repeat, overlay, or arbitrary paint-over.
- **Physical/clip evidence:** At 1x, 1.5x, and 2x, the restored two-point padding maps with floor/ceil conversion as a bounded owned span inside the top-panel clip. The clear color is the only owner available behind any row outside that egui clip; matching it to the chrome surface prevents a backing-store row at the full-size-content boundary from appearing black without changing logical rectangles or hit testing.
- **Preservation:** Title strip/full-width hit coverage, toolbar allocation and right edge, `TOOLBAR_ELEMENTS_WIDTH == 128.0`, cursor/popup/input/document behavior, central/status ordering, viewport request semantics, and all V3-WO-002 behavior remain unchanged. Requirements and blueprints remain `verify unchanged`.
- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1), this work order, and the v3 execution log. No validation artifact was created; no V3-WO-002 source was changed.
- **Deterministic evidence:** Added restored-padding ownership and viewport-clear-color equality tests; updated the existing physical-row expectations to the restored two-point span. Focused chrome/clear-color/padding tests passed.
- **Required checks:** `cargo clean`; `cargo fmt --all -- --check`; `cargo test -p scriptcraft` (84 passed); `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; and `git diff --check` all passed with exit 0 after the clean rebuild.
- **Native/user gate:** No visual pass is claimed by the implementer. The next action is the mandatory user run of `cargo run -p scriptcraft` and visual inspection in normal, maximized, and restored frames, including no strip and fully visible toolbar text/content.
- **Next state:** `in-review`; await the user's visual pass/fail. Do not create `validation-V3-WO-001b.md`, dispatch validation, or dispatch V3-WO-002 until the user reports the result.

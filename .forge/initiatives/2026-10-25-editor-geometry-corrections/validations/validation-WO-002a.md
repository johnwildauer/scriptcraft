# Validation — WO-002a Font and Profile Rendering Fix

## Validation metadata

- **Validation status:** `FAIL`
- **Work order:** [`WO-002a-font-and-profile-rendering-fix.md`](../work-orders/WO-002a-font-and-profile-rendering-fix.md:1)
- **Original work order:** [`WO-002-screenplay-width-and-leading.md`](../work-orders/WO-002-screenplay-width-and-leading.md:1)
- **Prior failing validation:** [`validation-WO-002.md`](validation-WO-002.md:1)
- **Initiative:** [`editor-geometry-corrections-v1.md`](../editor-geometry-corrections-v1.md:1)
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `debug-fresh-context-editor-geometry-corrections-WO-002a-validation-2026-10-25T22:35Z`
- **Fresh-context statement:** This validation was performed as a separate fresh-context session. I read [`05-validation.md`](../../../process/05-validation.md:1) and [`00-overview.md`](../../../process/00-overview.md:1), the original WO-002, the failing validation, WO-002a, the approved follow-up plan, design review, format reference, execution log, affected blueprints, changed source, and current implementation evidence. I did not implement fixes, did not validate WO-001 or unrelated work, and did not mark WO-002a or WO-002 complete.
- **Work-order state at validation:** `in-review`; WO-002a remains not complete and WO-002 remains not complete.

## Validation boundary and diagnosis

Only the two WO-002 hard stops were assessed: (1) the approved 12-point Courier-family renderer target/fallback, and (2) complete persisted profile ownership of role boxes, line advance, boundary gap, and font metadata across model persistence, layout, caret/hit testing, app rendering, pagination, and PDF.

Potential sources considered were: (1) the renderer still using the former scaled 10-point path; (2) the renderer using a fallback without preserving the approved base point size; (3) role boxes remaining hardcoded outside the profile; (4) line advance or boundary gap being replaced by app/layout constants; (5) caret and hit testing using a different character metric; (6) pagination or PDF reconstructing geometry/font metadata independently; and (7) alternate bounded profiles failing deterministic JSON round trips. The two most likely sources were consumer divergence in PDF/app code and loss of non-default boundary-gap values during pagination. Source inspection, focused tests, and the workspace suite were used as diagnostic logs.

## Comparison basis and inspected evidence

- The approved geometry/font contract in [`format-rules.md`](../format-rules.md:5), including 12-point Courier-family choices, 72 points/inch, explicit role boxes, single spacing, and one non-text boundary gap.
- The approved WO-002 geometry oracle and fix boundary in [`WO-002-screenplay-width-and-leading.md`](../work-orders/WO-002-screenplay-width-and-leading.md:22) and [`WO-002a-font-and-profile-rendering-fix.md`](../work-orders/WO-002a-font-and-profile-rendering-fix.md:16).
- Persisted profile fields and deterministic JSON encode/decode in [`PageProfile`](../../../../crates/scriptcraft-core/src/lib.rs:154), [`validate_profile()`](../../../../crates/scriptcraft-core/src/lib.rs:315), [`encode_envelope()`](../../../../crates/scriptcraft-core/src/lib.rs:377), and [`decode_project()`](../../../../crates/scriptcraft-core/src/lib.rs:595).
- Profile-derived layout, wrapping, role boxes, line rectangles, hit testing, carets, and pagination in [`ScreenplayLayout::build()`](../../../../crates/scriptcraft-core/src/layout.rs:88), [`ScreenplayLayout::hit_test()`](../../../../crates/scriptcraft-core/src/layout.rs:138), [`ScreenplayLayout::caret_rect()`](../../../../crates/scriptcraft-core/src/layout.rs:159), and [`ScreenplayLayout::place_element()`](../../../../crates/scriptcraft-core/src/layout.rs:240).
- Desktop selection/IME geometry and renderer font construction in [`ScriptCraftApp::paint_page()`](../../../../apps/scriptcraft/src/main.rs:278) and [`renderer_font()`](../../../../apps/scriptcraft/src/main.rs:805).
- PDF font and coordinate generation in [`export_pdf()`](../../../../crates/scriptcraft-core/src/exports.rs:68) and [`page_content()`](../../../../crates/scriptcraft-core/src/exports.rs:128).
- Focused default/alternate profile tests in [`named_profile_round_trips_bounded_alternate_geometry()`](../../../../crates/scriptcraft-core/src/lib.rs:657), [`alternate_profile_drives_role_wrap_line_and_caret_geometry()`](../../../../crates/scriptcraft-core/src/layout.rs:539), and [`renderer_uses_profile_point_size_and_deterministic_monospace_fallback()`](../../../../apps/scriptcraft/src/main.rs:1575).

The changed product files are limited to the authorized model/layout/export/app and blueprint/reference surface recorded by the implementer. No unrelated product behavior, UI pixel persistence, per-element formatting, override editor, role transition, or viewport-navigation change was found.

## Acceptance assessment

| WO-002a hard-stop obligation | Result | Evidence |
|---|---|---|
| Default named profile and bounded alternate profile round trips | **PASS** | The model persists font family/size/character metric, line advance, boundary gap, and seven role boxes; the default and alternate deterministic JSON regression passes. [`PageProfile`](../../../../crates/scriptcraft-core/src/lib.rs:154) is the durable seam. |
| Inch-to-point boundary and approved default geometry | **PASS** | [`POINTS_PER_INCH`](../../../../crates/scriptcraft-core/src/lib.rs:19) is 72; default paper/margins and role boxes match the approved 612 × 792 point oracle. |
| Role boxes drive layout, wrapping, caret/hit testing, and pagination | **PASS** | [`role_box()`](../../../../crates/scriptcraft-core/src/layout.rs:330), [`role_width()`](../../../../crates/scriptcraft-core/src/layout.rs:322), line rectangles, [`hit_test()`](../../../../crates/scriptcraft-core/src/layout.rs:138), [`caret_rect()`](../../../../crates/scriptcraft-core/src/layout.rs:159), and [`place_element()`](../../../../crates/scriptcraft-core/src/layout.rs:240) read the profile-derived values. Alternate role/wrap/caret/hit-test coverage passes. |
| Line advance and boundary gap are fully profile-driven | **FAIL** | [`place_element()`](../../../../crates/scriptcraft-core/src/layout.rs:247) reduces `boundary_gap` to `boundary_gap / line_advance`; a bounded alternate profile with boundary gap 10 and line advance 15 therefore becomes zero extra boundary units. The alternate regression expects a 30-unit jump, proving the stored value is not preserved as geometry. |
| App selection/IME/caret rendering and viewport zoom separation | **PASS** | [`paint_page()`](../../../../apps/scriptcraft/src/main.rs:278) uses layout line height/character width and profile font size; [`renderer_font()`](../../../../apps/scriptcraft/src/main.rs:805) applies the persisted base size before presentation scale. Existing viewport-scale tests pass and no viewport value is persisted. |
| Approved 12-point Courier-family target/fallback | **PASS WITH WARNINGS** | The default profile is Courier New at 12 points, and the app uses a documented deterministic monospace fallback at `profile.font_size * scale`; the focused renderer regression passes. The implementation does not attempt named-face selection, so native face selection remains unobserved and fallback-only in this environment. |
| PDF consumes profile-derived geometry and font metadata | **FAIL** | PDF text size is profile-derived in [`export_pdf()`](../../../../crates/scriptcraft-core/src/exports.rs:84), and line positions come from layout, but the PDF resource is hardcoded to `/BaseFont /Courier` in [`export_pdf()`](../../../../crates/scriptcraft-core/src/exports.rs:82). An alternate persisted `Courier Prime` profile therefore does not own PDF font metadata. |
| No UI pixels or per-element formatting | **PASS** | Profile fields are bounded format values; semantic elements contain role/text/ID only. No override editor, page-pixel persistence, or duplicated per-element formatting was added. |
| Required automated checks | **PASS** | Exact results are recorded below; all requested commands exited 0. |

## Exact command results

Executed from the repository root on 2026-10-25:

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | **PASS**, exit 0 |
| `cargo check --workspace` | **PASS**, exit 0 |
| `cargo test --workspace` | **PASS**, exit 0 — 23 app tests, 32 core unit tests, 5 WO-003 integration tests, 3 WO-008 integration tests, 0 doc-test failures; 63 tests passed, 0 failed |
| `cargo clippy --workspace --all-targets -- -D warnings` | **PASS**, exit 0 |
| `cargo build --workspace` | **PASS**, exit 0 |
| `git diff --check` | **PASS**, exit 0 for the tracked diff |
| Native smoke | **WARNING / UNAVAILABLE FOR INTERACTION** — bounded `cargo run -p scriptcraft` reached `target/debug/scriptcraft`, remained alive for 8 seconds, and was terminated with exit 143. No observable/interactable native window or visual/PDF inspection was available. No native success is claimed. This is not an independent hard stop under the project policy because manual desktop interaction is optional. |

## Findings

### EGC-WO002A-F-001 — Boundary-gap ownership is quantized and fails a bounded alternate profile

- **Severity:** hard-stop
- **Evidence:** [`PageProfile::boundary_gap`](../../../../crates/scriptcraft-core/src/lib.rs:166) persists the value, but [`ScreenplayLayout::place_element()`](../../../../crates/scriptcraft-core/src/layout.rs:247) computes `boundary_units` by integer-dividing it by `line_advance`. The focused alternate fixture sets `boundary_gap = 15` and `line_advance = 15`, so it does not expose the loss. The implementer’s round-trip fixture sets `boundary_gap = 10` and `line_advance = 14`, while no layout assertion proves that 10-unit boundary geometry survives. The approved contract requires the persisted boundary gap to remain the non-text spacing source, not to be silently rounded to whole line boxes.
- **Recommendation:** Preserve and apply the persisted boundary gap as point geometry independently of line-box count. Keep page-capacity accounting and line origins deterministic, and add an alternate-profile assertion where boundary gap is not a multiple of line advance so the exact gap is observed between adjacent elements and across page decisions.
- **Required next state:** `WO-002a` remains `in-review`; create a narrowly scoped follow-up fix work order under WO-002’s profile/layout boundary and require a fresh validation. Do not complete WO-002 through this validation.

### EGC-WO002A-F-002 — PDF still bypasses persisted font-family metadata

- **Severity:** hard-stop
- **Evidence:** [`PageProfile::font_family`](../../../../crates/scriptcraft-core/src/lib.rs:168) is persisted and the app consumes the profile font size, but [`export_pdf()`](../../../../crates/scriptcraft-core/src/exports.rs:82) always emits `/BaseFont /Courier`. The approved bounded profile accepts `Courier`, `Courier New`, and `Courier Prime` in [`validate_profile()`](../../../../crates/scriptcraft-core/src/lib.rs:315); an alternate `Courier Prime` profile therefore round-trips in the model but exports with different font metadata. The current PDF test changes font size only and does not assert alternate font-family propagation.
- **Recommendation:** Route the approved profile font-family identity through the PDF font resource, using the supported PDF family mapping/fallback contract, and add a bounded alternate-profile export assertion. Preserve the existing shared layout coordinates and deterministic fallback behavior.
- **Required next state:** `WO-002a` remains `in-review`; create a narrowly scoped follow-up fix work order under the approved profile/font/PDF boundary and require fresh validation. Do not complete WO-002.

### EGC-WO002A-W-001 — Native visual/font smoke unavailable

- **Severity:** warning
- **Evidence:** The bounded native launch reached the binary but this session exposed no interactable window; therefore named Courier face selection, fallback appearance, screen/PDF visual parity, and zoom presentation could not be visually inspected.
- **Recommendation:** Retain this as an honest manual-evidence limitation. Do not convert headless renderer/layout tests into native visual proof.
- **Required next state:** Note/follow-up only under current policy; it does not independently require a fix work order.

## Verdict and orchestrator next state

- **Verdict:** `FAIL`.
- **Reason:** The implementation fixes the former scaled 10-point path, persists the requested profile fields, routes most layout/caret/app geometry through the profile, and passes every required automated command. It does not satisfy the requested *full* profile-driven ownership: non-multiple alternate boundary gaps are reduced to zero whole-line units, and PDF font-family metadata remains hardcoded rather than profile-owned. These are concrete unmet hard-stop implementation criteria, not design preferences.
- **Orchestrator action:** Do not complete WO-002a. Create another narrowly scoped letter-suffixed fix work order for the two findings above, dispatch a new implementer, and then dispatch a fresh validator. After that fix passes, the orchestrator should complete the fix chain and then complete original WO-002. No unrelated work should be validated or changed.
- **Status guard:** This validator did not implement fixes, did not mark WO-002a complete, did not mark WO-002 complete, and did not validate unrelated work.

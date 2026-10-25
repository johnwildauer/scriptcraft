# Validation — WO-002 Screenplay Width and Leading

## Validation metadata

- **Validation status:** `FAIL`
- **Work order:** [`WO-002-screenplay-width-and-leading.md`](../work-orders/WO-002-screenplay-width-and-leading.md:1)
- **Initiative:** [`editor-geometry-corrections-v1.md`](../editor-geometry-corrections-v1.md:1)
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `debug-fresh-context-editor-geometry-corrections-WO-002-validation-2026-10-25T22:17:55Z`
- **Fresh-context statement:** This validation was performed as a separate fresh-context session. I read [`05-validation.md`](../../../process/05-validation.md:1) and [`00-overview.md`](../../../process/00-overview.md:1), then independently inspected the follow-up plan, approved [`design-review.md`](../reviews/design-review.md:1), approved geometry oracle in [`WO-002-screenplay-width-and-leading.md`](../work-orders/WO-002-screenplay-width-and-leading.md:22), [`format-rules.md`](../format-rules.md:1), execution log, changed-file record, implementation diff, changed model/layout/export/app files, and affected blueprints. I did not implement fixes, did not validate WO-001 or unrelated work, and did not mark WO-002 complete.
- **Work-order state at validation:** `in-review`; WO-002 remains not complete.

## Comparison basis

Only WO-002 was assessed. The comparison boundary was the approved point-based format contract and its shared model/layout/export/app behavior:

- `72 points/inch`; US Letter `612 × 792`; margins `108/72/72/72`.
- Scene/action/note `108..540`; character centered at `259` (3.6 inch target within the approved range); parenthetical `216..396`; dialogue `180..396`; transition `432..540`, right-aligned.
- Single-spaced wrapped lines and one non-text `12`-unit boundary gap for every adjacent element pair.
- Courier-family 12-point target, deterministic final line metric, named/versioned persisted profile, and a future override seam without UI pixels or per-element formatting.
- Deterministic source/caret mappings, pagination, PDF transforms, and viewport-scale separation.
- Required checks and durable-document classification.

Potential sources considered were: (1) incorrect inch-to-point conversion; (2) wrong stored margin or content edge; (3) role boxes being confused with page margins; (4) wrapping in points versus character cells; (5) missing or quantized boundary spacing; (6) screen/PDF/caret consumers using different geometry or fixed app constants; and (7) the persisted profile not carrying the complete future override seam or actual font contract. The most likely implementation risks were (a) consumers retaining pre-profile constants and (b) the format seam persisting only a subset of the explicitly overridable format fields. Source inspection and the focused tests validate the default geometry, but confirm those two risks in the implementation.

## Evidence inspected

- The approved oracle and implementer evidence in [`WO-002-screenplay-width-and-leading.md`](../work-orders/WO-002-screenplay-width-and-leading.md:22), including the final geometry table and changed-file record.
- The human-readable format contract in [`format-rules.md`](../format-rules.md:32).
- [`PageProfile`](../../../../crates/scriptcraft-core/src/lib.rs:146), its default conversion and persistence fields, validation, and JSON encode/decode paths.
- [`ScreenplayLayout::build()`](../../../../crates/scriptcraft-core/src/layout.rs:88), [`ScreenplayLayout::make_line()`](../../../../crates/scriptcraft-core/src/layout.rs:197), [`role_width()`](../../../../crates/scriptcraft-core/src/layout.rs:306), [`role_box()`](../../../../crates/scriptcraft-core/src/layout.rs:311), and [`place_element()`](../../../../crates/scriptcraft-core/src/layout.rs:228).
- The desktop renderer in [`ScriptCraftApp::paint_page()`](../../../../apps/scriptcraft/src/main.rs:280), including `FontId::monospace` and the `LINE_HEIGHT` selection rectangle.
- PDF geometry tests and transforms in [`exports.rs`](../../../../crates/scriptcraft-core/src/exports.rs:1), desktop viewport tests in [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1544), and the focused layout/model tests.
- The affected durable blueprints [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:111), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:86), and [`technical-context.md`](../../../blueprints/technical-context.md:1).

No clearly unrelated, deceptive, backdoor, malicious, or nefarious behavior was found. The changed implementation files and blueprint updates are generally within WO-002 scope.

## Acceptance assessment

| WO-002 obligation | Result | Evidence |
|---|---|---|
| Inch-defined point conversion | **PASS for default** | [`POINTS_PER_INCH`](../../../../crates/scriptcraft-core/src/lib.rs:19) is `72`; the default conversion yields `612 × 792` from 8.5 × 11 inches and `108/72/72/72` margins in [`PageProfile::default()`](../../../../crates/scriptcraft-core/src/lib.rs:162). |
| US Letter and margins | **PASS** | [`new_page()`](../../../../crates/scriptcraft-core/src/layout.rs:172) derives page/content bounds from the persisted profile; focused margin tests passed. |
| Role boxes and transition alignment | **PASS for default** | [`role_box()`](../../../../crates/scriptcraft-core/src/layout.rs:311) yields the approved default rectangles, including transition `432..540`; focused role-edge and note-inheritance tests passed. |
| Single-spaced wraps and 12-unit boundary gaps | **PASS for default** | [`place_element()`](../../../../crates/scriptcraft-core/src/layout.rs:228) applies one `boundary_gap / line_advance` unit before subsequent elements, producing a 24-unit origin difference for adjacent one-line elements with the default profile; focused spacing tests passed. |
| Courier-family target and final line metric | **FAIL** | Core exposes a descriptive [`FONT_FAMILY`](../../../../crates/scriptcraft-core/src/layout.rs:9), but the native renderer uses [`FontId::monospace`](../../../../apps/scriptcraft/src/main.rs:326) at `10.0 * scale`, not an explicit Courier/Courier New 12-point face. The fixed `7`-unit/`12`-unit metrics are deterministic defaults, but the actual rendered font contract is not established as the approved 12-point Courier-family target. |
| Named/versioned persisted profile | **PASS for persisted default fields** | [`PageProfile`](../../../../crates/scriptcraft-core/src/lib.rs:146) persists `name`, `format_version`, paper/margins, `line_advance`, and `boundary_gap`; native round-trip tests passed. |
| Future override seam without UI pixels/per-element formatting | **FAIL** | The approved reference requires explicit overridable role boxes as well as margins, leading, and gaps ([`format-rules.md`](../format-rules.md:39)). Role boxes remain hardcoded in [`role_box()`](../../../../crates/scriptcraft-core/src/layout.rs:311), and the desktop selection/IME renderer uses constant [`LINE_HEIGHT`](../../../../apps/scriptcraft/src/main.rs:320) instead of `layout.line_height`. A future profile override cannot replace all approved format fields consistently. No UI editor or per-element pixel formatting was added, which is aligned. |
| Source/caret/pagination/PDF/viewport invariants | **PASS for current defaults** | Layout source ranges/caret rectangles, page capacity, PDF transform parity, and viewport-scale independence tests passed. The shared layout remains the source of truth for these tested default paths. |
| Required checks | **PASS** | Exact results are recorded below; all five requested Cargo checks exited `0`. |
| Durable-document classification | **PASS** | The diff updates the affected technical/component/feature blueprints and verifies the existing requirements remain sufficient; this is consistent with WO-002's conditional requirements classification and required blueprint update. |

## Exact check results

Executed from the repository root on 2026-10-25:

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | **PASS**, exit 0 |
| `cargo check --workspace` | **PASS**, exit 0 |
| `cargo test --workspace` | **PASS**, exit 0 — 22 app tests, 29 core unit tests, 5 WO-003 integration tests, 3 WO-008 integration tests, 0 doc-tests; 59 tests passed, 0 failed |
| `cargo clippy --workspace --all-targets -- -D warnings` | **PASS**, exit 0 |
| `cargo build --workspace` | **PASS**, exit 0 |
| Native smoke | **WARNING / UNAVAILABLE FOR INTERACTION** — `cargo run -p scriptcraft` remained running during an 8-second bounded launch attempt, but this context exposed no observable/interactable native window. The process was terminated by the bounded subprocess attempt. No native visual/font/geometry result is claimed. This is not the hard-stop finding because the project policy treats desktop manual interaction as optional. |

## Findings

### EGC-WO002-F-001 — Native renderer does not establish the approved 12-point Courier-family target

- **Severity:** hard-stop
- **Evidence:** [`format-rules.md`](../format-rules.md:6) and the approved oracle require a 12-point Courier-family target. [`FONT_FAMILY`](../../../../crates/scriptcraft-core/src/layout.rs:9) is only a descriptive string, while [`paint_page()`](../../../../apps/scriptcraft/src/main.rs:280) constructs [`FontId::monospace`](../../../../apps/scriptcraft/src/main.rs:326) with a `10.0` point-size argument scaled for the viewport. No explicit Courier/Courier New family selection or measured final glyph/line metric is present.
- **Recommendation:** Make the native text renderer select the approved Courier-family face at the approved 12-point base size, or record and use a measured fixed fallback that is explicitly approved as the final metric. Keep viewport scale as a display transform only and add a regression that checks the renderer's selected family/base metric against the profile/layout contract.
- **Required next state:** Create a letter-suffixed fix work order, such as `WO-002a-courier-renderer-and-profile-seam.md`, scoped to the approved WO-002 geometry/font contract; keep WO-002 `in-review` and not complete until the fix validates.

### EGC-WO002-F-002 — Persisted override seam does not cover role boxes or all consumers

- **Severity:** hard-stop
- **Evidence:** [`format-rules.md`](../format-rules.md:39) requires future overrides to replace explicit margins, role boxes, leading, and boundary gaps. [`PageProfile`](../../../../crates/scriptcraft-core/src/lib.rs:146) persists margins, line advance, and boundary gap but no role-box definitions. [`role_box()`](../../../../crates/scriptcraft-core/src/layout.rs:311) hardcodes every role rectangle. Further, [`paint_page()`](../../../../apps/scriptcraft/src/main.rs:320) uses `LINE_HEIGHT` instead of `layout.line_height`, so a persisted line-advance override would not fully reach the native selection rendering path.
- **Recommendation:** Add bounded, validated, versioned profile fields for role boxes (or an equivalent explicit role-geometry table) and route layout, wrapping, caret, selection, IME, pagination, PDF, and viewport consumers through the same profile-derived geometry. Add tests using non-default profile values to prove that overrides remain deterministic and do not add UI pixels or per-element formatting.
- **Required next state:** Create the same fix WO (or a narrowly linked follow-up fix WO) within WO-002's approved boundary; do not mark the original complete until the override seam and all consumers validate.

### EGC-WO002-W-001 — Native smoke was unavailable

- **Severity:** warning
- **Evidence:** The bounded `cargo run -p scriptcraft` attempt launched a process that remained active for 8 seconds, but no interactable window or visual capture was available.
- **Recommendation:** Preserve this limitation and do not treat headless tests as native visual proof. Repeat only in a capable desktop interaction session.
- **Required next state:** Note only; it does not independently block validation under the current project policy.

## Verdict and next state

- **Verdict:** `FAIL`.
- **Implementation alignment:** The default point geometry, role rectangles, spacing behavior, persistence round-trip, source/caret/pagination/PDF/viewport invariants, checks, and durable blueprint classification are evidenced and pass. However, the approved Courier-family renderer contract and the complete future profile override seam are plainly unmet implementation criteria, not design preferences.
- **Orchestrator action:** Create a letter-suffixed WO-002 fix work order and dispatch it to a new implementer session, followed by a fresh validation session. Do not complete WO-002 and do not claim native smoke evidence.
- **Status guard:** This validator did not mark WO-002 complete and did not validate unrelated work.

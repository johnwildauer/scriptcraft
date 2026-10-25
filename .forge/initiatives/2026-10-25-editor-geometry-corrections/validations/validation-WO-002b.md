# Validation — WO-002b Profile Gap and PDF Font Fix

## Validation metadata

- **Validation status:** `PASS WITH WARNINGS`
- **Work order:** [`WO-002b-profile-gap-and-pdf-font-fix.md`](../work-orders/WO-002b-profile-gap-and-pdf-font-fix.md:1)
- **Original work order:** [`WO-002-screenplay-width-and-leading.md`](../work-orders/WO-002-screenplay-width-and-leading.md:1)
- **Prior failing validation:** [`validation-WO-002a.md`](validation-WO-002a.md:1)
- **Initiative:** [`editor-geometry-corrections-v1.md`](../editor-geometry-corrections-v1.md:1)
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `debug-fresh-context-editor-geometry-corrections-WO-002b-validation-2026-10-25T22:40Z`
- **Fresh-context statement:** This validation was performed as a separate fresh-context session. I read [`05-validation.md`](../../../process/05-validation.md:1), [`00-overview.md`](../../../process/00-overview.md:1), the original [`WO-002-screenplay-width-and-leading.md`](../work-orders/WO-002-screenplay-width-and-leading.md:1), [`WO-002a-font-and-profile-rendering-fix.md`](../work-orders/WO-002a-font-and-profile-rendering-fix.md:1), this [`WO-002b-profile-gap-and-pdf-font-fix.md`](../work-orders/WO-002b-profile-gap-and-pdf-font-fix.md:1), the failing [`validation-WO-002a.md`](validation-WO-002a.md:1), the approved geometry oracle, [`format-rules.md`](../format-rules.md:1), [`execution-log.md`](../execution-log.md:1), the relevant implementation diff, and the changed layout/export/reference evidence. I did not implement fixes, did not validate unrelated work, and did not mark any work order complete.
- **Work-order state at validation:** `in-review`; WO-002b, WO-002a, and original WO-002 remain not complete.

## Validation boundary and diagnosis

Only WO-002b was assessed, specifically:

1. exact preservation of bounded non-multiple boundary gaps through profile persistence and layout placement;
2. deterministic PDF font mapping from persisted Courier-family metadata with safe fallback;
3. unchanged default geometry and spacing;
4. source, caret, pagination, PDF, and screen parity; and
5. absence of unrelated scope.

Possible sources considered were: (1) integer quantization of a persisted gap; (2) loss of the gap during profile JSON round-trip; (3) page-capacity bookkeeping diverging from the exact layout cursor; (4) PDF retaining a hardcoded font resource; (5) unsafe or nondeterministic mapping of unknown font metadata; (6) screen/caret/PDF consumers bypassing the shared profile/layout; and (7) unrelated files or behavior being changed. The two most likely sources, based on [`validation-WO-002a.md`](validation-WO-002a.md:63), were gap quantization and the hardcoded PDF `/BaseFont`. Source inspection, focused regression tests, the full workspace checks, and a bounded native launch were used as diagnostic evidence.

## Comparison basis and inspected evidence

- The approved point-based oracle in [`WO-002-screenplay-width-and-leading.md`](../work-orders/WO-002-screenplay-width-and-leading.md:22): 72 points/inch, US Letter `612 × 792`, margins `108/72/72/72`, approved role boxes, 12-unit default line advance, and one non-text gap between adjacent elements.
- The durable format reference in [`format-rules.md`](../format-rules.md:5): 12-point Courier-family metadata and no viewport pixels as format inputs.
- The prior hard stops in [`validation-WO-002a.md`](validation-WO-002a.md:63): integer division of `boundary_gap / line_advance` and a hardcoded PDF `/BaseFont /Courier`.
- The implementation diff in [`layout.rs`](../../../../crates/scriptcraft-core/src/layout.rs:241): page placement now adds the persisted boundary gap directly to the exact point cursor and keeps logical page-capacity accounting separate.
- The PDF mapping in [`exports.rs`](../../../../crates/scriptcraft-core/src/exports.rs:68): `export_pdf()` reads the persisted profile family through [`pdf_font_name()`](../../../../crates/scriptcraft-core/src/exports.rs:129), maps the three allowed Courier-family values deterministically, and falls back to `Courier` for unknown input.
- Focused regressions in [`layout.rs`](../../../../crates/scriptcraft-core/src/layout.rs:592) and [`exports.rs`](../../../../crates/scriptcraft-core/src/exports.rs:217): non-multiple gap geometry, default/alternate persisted font metadata, PDF resource mapping, fallback behavior, default spacing, PDF transforms, source ranges, caret geometry, and pagination.
- The changed-file record in [`WO-002b-profile-gap-and-pdf-font-fix.md`](../work-orders/WO-002b-profile-gap-and-pdf-font-fix.md:81) and execution evidence in [`execution-log.md`](../execution-log.md:36).

The changed product surface is limited to the authorized core layout and PDF export files, plus the work-order and execution evidence. No app navigation, canonical screenplay text, persistence schema, override UI, approved default role geometry, or unrelated product behavior was changed by WO-002b.

## Acceptance assessment

| WO-002b obligation | Result | Evidence |
|---|---|---|
| Non-multiple bounded boundary gaps survive persistence and layout | **PASS** | Profile persistence already round-trips bounded geometry; [`place_element()`](../../../../crates/scriptcraft-core/src/layout.rs:241) adds `boundary_gap` directly to the point cursor rather than dividing it by `line_advance`. The focused fixture uses `line_advance = 14` and `boundary_gap = 10` and observes the exact 24-point adjacent-element origin difference. |
| Deterministic PDF font mapping from persisted metadata | **PASS** | [`export_pdf()`](../../../../crates/scriptcraft-core/src/exports.rs:68) derives the PDF resource from persisted `font_family`; `Courier`, `Courier New`, and `Courier Prime` map deterministically, while unknown input maps safely to `Courier`. The alternate `Courier Prime` JSON round-trip/PDF assertion passes. |
| Default geometry and spacing unchanged | **PASS** | Default role edges, US Letter bounds, margins, 12-unit line height, 12-unit boundary gap, and adjacent one-line 24-unit origin difference remain asserted in [`layout.rs`](../../../../crates/scriptcraft-core/src/layout.rs:506) and [`layout.rs`](../../../../crates/scriptcraft-core/src/layout.rs:608). |
| Source/caret/pagination/PDF/screen parity | **PASS** | Existing source mapping, hit testing, caret rectangles, page-capacity/reset behavior, PDF coordinate transforms, persisted profile consumers, and viewport-scale independence tests remain green. The exact PDF transform assertions are in [`exports.rs`](../../../../crates/scriptcraft-core/src/exports.rs:230); screen renderer/profile parity remains covered by [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1575). |
| No unrelated scope or rogue behavior | **PASS** | The diff is confined to the authorized layout/export correction and its focused evidence. No deceptive, backdoor, malicious, nefarious, navigation, canonical-text, schema, or override-UI change was found. |
| Required automated checks | **PASS** | All six requested commands exited 0; exact results are recorded below. |
| Native smoke | **WARNING / UNAVAILABLE FOR INTERACTION** | The bounded `cargo run -p scriptcraft` launch reached `target/debug/scriptcraft` and remained alive for 8 seconds, but no observable/interactable native window was available in this session. It was terminated with exit 143. No native visual or interactive parity is claimed. This is optional evidence under the applicable project policy and is not an independent hard stop. |

## Exact command results

Executed from the repository root on 2026-10-25:

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | **PASS**, exit 0 |
| `cargo check --workspace` | **PASS**, exit 0 |
| `cargo test --workspace` | **PASS**, exit 0 — 23 app tests, 34 core unit tests, 5 WO-003 integration tests, 3 WO-008 integration tests, 0 doc-test failures; 65 tests passed, 0 failed |
| `cargo clippy --workspace --all-targets -- -D warnings` | **PASS**, exit 0 |
| `cargo build --workspace` | **PASS**, exit 0 |
| `git diff --check` | **PASS**, exit 0 |
| Native smoke | **WARNING / UNAVAILABLE FOR INTERACTION** — bounded launch remained alive after 8 seconds and was terminated with exit 143; no native window or visual inspection was available |

## Findings

### EGC-WO002B-W-001 — Native visual interaction was unavailable

- **Severity:** warning
- **Evidence:** The bounded [`cargo run -p scriptcraft`](../../../../Cargo.toml:1) attempt reached the executable and produced no runtime error, but this validation session exposed no interactable native window. The process was terminated after 8 seconds with exit 143.
- **Recommendation:** Retain this as an honest manual-evidence limitation. Do not treat headless layout/PDF tests as visual proof of native screen rendering or named-face appearance.
- **Required next state:** Note/follow-up only. It does not require another fix work order under the current optional-manual-gate policy.

No hard-stop findings were identified.

## Verdict and orchestrator next state

- **Verdict:** `PASS WITH WARNINGS`.
- **WO-002b result:** WO-002b passes its authorized implementation boundary. Both hard stops from [`validation-WO-002a.md`](validation-WO-002a.md:63) are resolved: non-multiple persisted gaps are applied as exact point geometry, and PDF font resources are profile-mapped with deterministic fallback.
- **Fix-chain result:** The original WO-002 fix chain can close from a validation perspective after the orchestrator records this result and performs the normal status transitions. The native warning remains documented and does not independently block closure.
- **Status guard:** This validator did not implement fixes, did not mark WO-002b, WO-002a, or WO-002 complete, and did not validate unrelated work.

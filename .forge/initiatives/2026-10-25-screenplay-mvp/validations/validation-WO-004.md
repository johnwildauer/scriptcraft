# Validation: WO-004 — Deterministic Layout and Display

## Validation metadata

- **Review type:** `validation`
- **Status:** `complete`
- **Validator:** Independent fresh-context validator
- **Validator session/agent ID:** `validation-session-2026-10-25-wo-004`
- **Session exception:** `none`
- **Date:** 2026-10-25
- **Initiative:** [`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md)
- **Validation file:** `validation-WO-004.md`
- **Validated work order:** [`WO-004-layout-display.md`](../work-orders/WO-004-layout-display.md)
- **Compared commit/diff:** No commit exists; comparison used the execution-log changed-file record and direct source inspection.
- **Fix chain:** `none`

## Authorized scope and comparison basis

The approved scope is the dependency-free fixed-profile layout/display path:
role-aware wrapping, deterministic integer geometry, page assignment, title-page
separation, keep-with-next behavior, bounded oversized-element handling, source
byte ranges, caret/hit-test mappings, structural tests, and model integration.
The comparison covered the approved work order, initiative requirements and
blueprints, validation process, execution log, [`layout.rs`](../../../crates/scriptcraft-core/src/layout.rs), and its export/model integration in [`lib.rs`](../../../crates/scriptcraft-core/src/lib.rs). The reported changed files are within scope; no UI, persistence, export, or unrelated behavior was introduced.

## Diagnostic hypotheses and evidence

The review considered these plausible failure sources:

1. Wrapping could split UTF-8, produce non-contiguous or incorrect source byte ranges, or mutate canonical text.
2. Role indents/measures could exceed content bounds or ignore the fixed profile and margins.
3. Pagination could assign lines to the wrong page, create blank pages, or loop on oversized elements.
4. Title-page records could mix with screenplay pages or fail to preserve separation.
5. Keep-with-next could be missing or fail at a page boundary.
6. Hit testing could return interior UTF-8 offsets or caret rectangles unrelated to the source line.
7. Repeated layout could vary for identical model/profile inputs.

The two highest-risk areas were reduced to (a) source/geometry correctness across
Unicode and every role and (b) pagination termination/repeatability. Both were
validated by source inspection, the focused tests, and an independent temporary
behavioral probe compiled against the current library. The probe emitted
`WO-004 independent structural probe: PASS (17 pages)` and was outside the
repository; it did not modify product code.

## Acceptance evidence

### Source and model integration inspection

- [`ScreenplayLayout::build()`](../../../crates/scriptcraft-core/src/layout.rs:87) validates the model and profile, pins the fixed metrics, creates a separate title page when present, and derives screenplay pages from the ordered semantic elements.
- [`Project::validate()`](../../../crates/scriptcraft-core/src/lib.rs:198) validates the project/profile before layout; [`ScreenplayElement::is_boundary()`](../../../crates/scriptcraft-core/src/lib.rs:118) supplies the model's UTF-8 boundary rule.
- [`role_indent_for()`](../../../crates/scriptcraft-core/src/layout.rs:311) and [`role_width()`](../../../crates/scriptcraft-core/src/layout.rs:297) constrain role boxes to the physical content width. [`new_page()`](../../../crates/scriptcraft-core/src/layout.rs:168) derives paper and content bounds from the fixed page profile.
- [`wrap_element()`](../../../crates/scriptcraft-core/src/layout.rs:334) and [`wrap_segment()`](../../../crates/scriptcraft-core/src/layout.rs:370) preserve source byte offsets while splitting only at character boundaries; no canonical model string is changed.
- [`place_element()`](../../../crates/scriptcraft-core/src/layout.rs:223) applies page capacity, keep-with-next minimum space, page assignment, and bounded per-element wrapping. Elements larger than a page are split rather than generating an empty-page loop.
- [`hit_test()`](../../../crates/scriptcraft-core/src/layout.rs:122) maps geometry to a source boundary, while [`caret_rect()`](../../../crates/scriptcraft-core/src/layout.rs:143) rejects invalid/interior offsets and returns geometry only for a matching line.

### Independent structural probe

The probe exercised all seven roles, a title page, Unicode text (`é🙂abc`),
small wrapping/page capacities, repeated layout equality, page/line assignment,
paper/content bounds, role-box bounds, keep-with-next flags, hit-test mapping,
caret round trip, rejection of an interior UTF-8 offset, and a 100-character
oversized element. It verified 17 non-empty pages for the oversized case and
completed without a panic or blank-page loop.

### Focused implementation tests

The five layout tests passed: bounded wrapping/source mapping; fixed margins,
bounds, and role boxes; title-page separation and keep rules; oversized-element
termination; and repeated-layout equality with hit/caret mapping.

### Workspace quality gates

All requested gates passed from this fresh validation context:

- `cargo fmt --all -- --check`
- `cargo check --workspace`
- `cargo test --workspace -- --nocapture` — 15 core tests passed; app and doc-test targets passed with no failures.
- `cargo clippy --workspace --all-targets -- -D warnings`

## Findings

### Blockers

- None. The implementation meets the applicable WO-004 acceptance behaviors,
  remains within the authorized scope, and has no clearly deceptive, backdoor,
  malicious, nefarious, or otherwise rogue behavior.

### Warnings

- The repository has no commits, so a commit-based diff is unavailable. The
  execution-log changed-file record and direct source inspection provide a
  meaningful comparison basis; this does not block validation.
- The visible desktop/manual preview gate is not applicable to this work order's
  headless backend-neutral scope; no UI framework is selected. Structural output
  was inspected through the independent probe instead.

### Notes

- The fixed metrics are intentionally integer/backend-neutral rather than a
  platform-rendered font measurement, matching the approved initial profile and
  the viewport-independent contract.
- The test/probe uses derived layout records only; semantic project text remains
  the model source of truth.

## Decision

- **Outcome:** `pass`
- **Required next state:** Set WO-004 to `complete` and retain this linked validation evidence.
- **Fix work order required:** `no`

# Work Order: WO-002 — Screenplay Width and Leading

## Metadata

- **Status:** `complete`
- **Assignee:** Fresh Code-mode implementer after WO-001 validation
- **Phase/order:** `2 of 2`
- **Initiative:** [`../editor-geometry-corrections-v1.md`](../editor-geometry-corrections-v1.md)
- **Generated filename:** `WO-002-screenplay-width-and-leading.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-002.md`](../validations/validation-WO-002.md)
- **Fixes:** passed fix chain [`WO-002a-font-and-profile-rendering-fix.md`](WO-002a-font-and-profile-rendering-fix.md) → [`WO-002b-profile-gap-and-pdf-font-fix.md`](WO-002b-profile-gap-and-pdf-font-fix.md), validated by [`validation-WO-002b.md`](../validations/validation-WO-002b.md)
- **Implementer session/agent ID:** `7F7A8C2E-5A45-4D5B-9D1F-2C0C4C1E8A72`
- **Session exception:** `none`

## Summary

Measure and correct the screenplay page's effective text width and visible vertical geometry. The work must distinguish stored page margins from role-specific text measures, then define numeric same-element leading and inter-element spacing for each role group before changing the shared layout contract.

The user-provided reference for this investigation is [`format-rules.md`](../format-rules.md). It is a formatting target/reference, not permission to claim exact industry parity without measuring the current fixed-metric renderer and reconciling the units.

## Proposed geometry oracle — design-only amendment

This section is the approved oracle for WO-002. It consolidates the user-provided
reference with the current fixed metrics. Layout definitions are inch-based and
converted to points (`72 × inches`) at the geometry boundary; viewport pixels
must not become the format source of truth.

### Unit and font assumptions

- Treat one layout unit as one typographic point: `1 unit = 1 pt = 1/72 inch`.
- Treat the page as US Letter: `8.5 × 11 in = 612 × 792 layout units`.
- Use a 12-point Courier-family face, preferring Courier or Courier New as
  available on the system. The exact installed face and its measured glyph
  advance are renderer concerns; the oracle must not silently reinterpret
  points as device pixels or viewport-scaled units.
- The current source instead uses the named fixed face `ScriptCraft Fixed
  Metrics`, a seven-unit character cell, and a twelve-unit line box. Those are
  current evidence, not the proposed font contract. The implementation must
  reconcile actual 12-point Courier-family metrics with the point-based page
  coordinates before changing wrapping or caret geometry.

### Current-versus-proposed table

Current role values below are derived from the current `90/72/72/72` margins,
the seven-unit cell, and the role indents/measures in the source. A target range
is preserved as a range where the reference itself uses `around` rather than
specifying a single edge.

| Geometry item | Current fixed geometry | Proposed reference oracle | Decision/interpretation |
|---|---|---|---|
| Paper | `612 × 792` units (`8.5 × 11 in`) | `612 × 792` units | Retain; point mapping is explicit. |
| Stored margins L/R/T/B | `90/72/72/72` units (`1.25/1/1/1 in`) | `108/72/72/72` units (`1.5/1/1/1 in`) | Change target left margin only if approved. |
| Content box | `x=90, y=72, w=450, h=648`; right edge `540` | `x=108, y=72, w=432, h=648`; right edge `540` | Page right edge remains the one-inch margin edge. |
| Scene heading | `x=90`, `385` units, edge `475` | `x=108` to `540` (`1.5` to `7.5 in`) | Treat as main-margin/action-width geometry unless separately approved. |
| Action | `x=90`, `385` units, edge `475` | `x=108` to `540` (`1.5` to `7.5 in`) | Fill the target content width; do not represent the residual as margin. |
| Note | `x=90`, `385` units, edge `475` | Inherit action: `x=108` to `540` (`1.5` to `7.5 in`) | Approved inheritance. |
| Character | `x=195` (`2.708 in`), `175` units, edge `370` | Left anchor around `252–266` units (`3.5–3.7 in`); right edge not specified | Preserve the range; exact anchor and measure require approval. |
| Parenthetical | `x=174` (`2.417 in`), `210` units, edge `384` | `x=216` to `396` (`3.0` to `5.5 in`) | Target box is explicit. |
| Dialogue | `x=160` (`2.222 in`), `245` units, edge `405` | `x=180` to `396` (`2.5` to `5.5 in`) | Target box is explicit. |
| Transition | `x=300` (`4.167 in`), `238` units, edge `538` | `x=432` (`6.0 in`) to right edge `540`, right aligned | Target right alignment is explicit; exact box width follows from the page edge. |
| Main-role residual to content edge | Action/scene/note: `65` units; paper-edge residual `137` units | `0` units to content edge; `72` units to paper edge | Separates role measure from stored page margin. |

### Proposed spacing semantics

- Wrapped lines within one element are single-spaced: consecutive line origins
  use one line advance, with no extra blank-line advance.
- The current measurable baseline is a `12`-unit line box and `12`-unit
  same-element origin advance. The proposed oracle retains that as the initial
  point-based 12-point baseline unless font measurement demonstrates that the
  line box must be separated from the glyph extent.
- A blank line equivalent between distinct element boundaries means one extra
  line advance after the prior element's final line box before the next
  element's first line origin. Under the current 12-unit baseline this is a
  `12`-unit inter-element gap, not a literal newline or inserted canonical text;
  the resulting boundary-to-boundary origin difference is `24` units for
  adjacent one-line elements.
- Apply the approved blank-line-equivalent gap to every adjacent role pair,
  including note and transition boundaries, without inserting canonical text.

### Approved implementation decisions

1. Interpret `around 3.5–3.7 in` as an allowed character anchor range and use
   the deterministic center, `3.6 in`, unless measured renderer constraints
   require another point within the approved range.
2. Notes inherit action's `1.5–7.5 in` box.
3. Apply one blank-line-equivalent gap to every role transition.
4. Start with the point-based 12-unit line advance, but permit measured
   installed Courier-family metrics to replace it when geometry tests show that
   the glyph extent and line box do not agree. Record the selected metric in the
   durable format reference and profile contract.

## In scope

- Establish a geometry table for paper bounds, stored margins, content box, role indents, role text widths, effective right edge, line box height, glyph/leading relationship, and inter-element gaps.
- Investigate the current 55-cell role cap against the 450-unit content width and select an approved approach: derive main role widths from content width, adjust fixed cell metrics, or explicitly retain a narrower screenplay measure with a documented rationale.
- Define role-aware spacing for scene heading, action, character, dialogue, parenthetical, transition, and note, including same-element wrapped lines versus adjacent elements.
- Keep screen layout, caret rectangles, pagination, and PDF transforms on one deterministic geometry source.
- Add structural tests and update durable blueprints/requirements only when the final contract changes.

## Out of scope

- Arbitrary user formatting, new profile/schema migrations without approval, font fallback systems, or unrelated navigation/input behavior.

## Requirements

- **REQ-WRITING-LOOP-002:** “As an individual screenwriter, I want continuous writing and fixed-page preview to agree about semantic content and layout inputs, so that I can write comfortably and inspect a predictable page result.”
- **AC-WRITING-002.1:** “Given the same project and fixed page profile, the system shall derive stable page structure and source mappings independent of window zoom.”
- **AC-WRITING-002.2:** “Given role-aware layout, the system shall apply role measures and keep relationships without inserting spaces or tabs into canonical text.”
- **NFR-WRITING-003:** “Native serialization and structural pagination outputs shall be deterministic for the same semantic project/profile inputs.”

## Blueprints

- [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md) — layout owns fixed profile metrics, role measures, source mappings, and caret geometry.
- [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md) — screen and PDF consume the same page/display inputs.
- [`technical-context.md`](../../../blueprints/technical-context.md) — current fixed metrics are provisional evidence to verify, not an immutable product promise.

## Implementation plan

1. Add a measurement fixture that exposes current effective margins/widths and baseline/gap positions for every role and representative wraps.
2. Agree on numeric geometry targets in design review before production changes; explicitly record whether the right edge is page margin or a role measure boundary.
3. Implement the smallest profile/layout change, preserving source ranges and caret hit/caret agreement.
4. Add screen-transform and PDF parity tests, page-capacity tests, and regression fixtures for role transitions/keep rules.

## Files and systems

- **Create:** focused layout geometry/spacing tests and fixtures.
- **Update:** [`crates/scriptcraft-core/src/layout.rs`](../../../crates/scriptcraft-core/src/layout.rs), [`crates/scriptcraft-core/src/lib.rs`](../../../crates/scriptcraft-core/src/lib.rs) only if profile persistence changes, [`crates/scriptcraft-core/src/exports.rs`](../../../crates/scriptcraft-core/src/exports.rs), and [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs) only for a proven renderer transform mismatch.
- **Reference:** [`format-rules.md`](../format-rules.md).
- **Durable format contract:** keep [`format-rules.md`](../format-rules.md) as the human-readable canonical reference and add/retain a named persisted format/profile seam so future project overrides can reference and replace these defaults without embedding visual pixels in elements. Override UI is out of scope.
- **Avoid changing:** canonical screenplay text, role-transition semantics, and viewport zoom behavior.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; `cargo check --workspace`; `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo build --workspace`.
- **Manual/external checks:** inspect representative scene heading/action/character/dialogue/parenthetical/transition blocks at a known scale and compare visible right edge, leading, inter-element gaps, caret height, page breaks, and PDF output.
- **Acceptance test:** Given the approved geometry table, when the same project is rendered on screen and exported, then effective role text widths and right margin are intentional, same-element lines show the approved leading, role-to-role spacing matches the approved table, page count/source mappings/caret rectangles remain deterministic, and viewport zoom does not change print geometry.

## Durable-document impact

- **Requirements impact:** `update` if user-visible geometry becomes a frozen requirement; otherwise `verify unchanged`
- **Blueprint impact:** `update`
- **Affected documents/IDs:** [`technical-context.md`](../../../blueprints/technical-context.md), [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md), and conditionally [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md) if screen/PDF shared behavior changes.
- **Rationale and evidence:** The current blueprint records fixed metrics but not a role-specific leading/gap table or the distinction between content width and role measure. Final classification depends on the approved geometry and diff.
- **Authorized documentation changes:** Record the final numeric geometry table and test evidence; do not claim industry-standard parity unless directly measured and approved.

## Completion checklist

- [x] Geometry targets are approved and recorded.
- [x] Implementation is complete within scope.
- [x] Tests/checks and required gates have evidence.
- [x] Work order is `in-review` before fresh validation; it remains out of `complete` pending the separate validator.

## Implementer completion evidence — 2026-10-25

- **Final geometry decisions:** Format boundary uses 72 points/inch and US Letter `612 × 792`; default profile is named/versioned `us_letter_screenplay` v1 with margins left/right/top/bottom `108/72/72/72`. Scene heading, action, and note use `x=108..540`; character uses deterministic `x=259` (3.597 inches, within the approved 3.5–3.7 inch range); parenthetical uses `216..396`; dialogue uses `180..396`; transition uses right-aligned `432..540`. Wrapped lines use the persisted 12-unit line advance; every distinct-element boundary consumes one persisted 12-unit non-text gap. Notes inherit action geometry. Fixed seven-unit character cells remain the deterministic renderer metric for the 12-point Courier-family target; no measured installed-font replacement was required.
- **Durable profile seam:** Native `PageProfile` now persists the named identity, `format_version`, bounded paper/margin values, `line_advance`, and `boundary_gap`; elements remain semantic and do not carry UI pixels or duplicated formatting. No override editor was added. [`format-rules.md`](../format-rules.md) remains the human-readable durable reference.
- **Changed files:** [`crates/scriptcraft-core/src/lib.rs`](../../../../crates/scriptcraft-core/src/lib.rs), [`crates/scriptcraft-core/src/layout.rs`](../../../../crates/scriptcraft-core/src/layout.rs), [`crates/scriptcraft-core/src/exports.rs`](../../../../crates/scriptcraft-core/src/exports.rs), [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs), [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md), [`technical-context.md`](../../../blueprints/technical-context.md).
- **Focused evidence:** Layout tests cover paper/margins/content bounds, all approved role edges and note inheritance, same-element versus boundary advance, source/caret mapping, and page capacity. Export tests assert PDF transforms consume the same rectangles and boundary spacing. Desktop tests retain viewport-scale independence. Native round-trip tests cover the new versioned profile fields.
- **Validation attempts:** `cargo test --workspace` passed after implementation (22 app tests, 29 core unit tests, 5 WO-003 integration tests, 3 WO-008 integration tests, 0 doc tests). Required remaining checks are recorded in the execution log and were run before this status transition.
- **Comparable diff:** The implementation is limited to shared profile serialization/layout/export geometry, the existing desktop geometry expectation, focused regressions, and authorized durable blueprints; canonical text/source mappings and viewport zoom inputs were not changed.

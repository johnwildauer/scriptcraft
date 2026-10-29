# Work Order: WO-005 — Status Content and Horizontal Clipping

## Metadata

- **Status:** `complete` — limited aggregate directional-alignment check passed; this is not comprehensive code validation.
- **Assignee:** Dedicated WO-005 coding implementer; actual host dispatch ID `01a185e7-91a8-74b8-ae3b-570218150a31` (reconciled from the user's handoff; no internal session ID is inferred).
- **Phase/order:** 1 of 4 in the user-directed follow-on batch after WO-004.
- **Initiative:** [`status-bar-v2.md`](../status-bar-v2.md)
- **Dependencies:** WO-004 is complete; no other batch WO is a dependency.
- **Next:** WO-006 after this implementation handoff.
- **Fixes:** none; new scoped change.

## Summary

Remove the status bar's displayed cumulative `Frames: …`, `Path: …` (including the unsaved-path label), and initial `In-memory demo project` placeholder. Retain Diagnostics for now and record that it is intended for pre-release removal; do not remove it in this WO. Prevent status labels, long text, icons, or changing counters from wrapping or increasing footer height: clip content horizontally to its existing allocation while retaining reasonable page/zoom controls and current geometry.

## In scope

- Remove the three specified status-bar display items/placeholders from the rendered presentation. Do not remove frame/path/session/diagnostic data when used elsewhere; do not remove counters from the retained Diagnostics disclosure.
- Keep clean/unsaved indication, viewport-visible page, meaningful operation/error feedback, the current Diagnostics disclosure, and existing 75–300% zoom slider/presets and accessibility.
- Suppress the initial in-memory-demo placeholder in the status surface; continue to show meaningful status/errors from real operations. Do not discard actual errors.
- Use single-line/no-wrap status content clipped at allocated horizontal bounds. Long strings, icons, and dynamic widths/counters must not alter footer height or intrude into neighboring regions. Respect the existing three-region composition and reasonable page/zoom geometry.
- Keep the Diagnostics disclosure and its privacy-safe summary; when expanded, its summary is one clipped line. Expansion may add its intentional fixed row, but changing diagnostic counters or text must not add rows or further grow the footer.
- Add rendered regressions using the actual status renderer at wide, compact, breakpoint-adjacent, and narrow widths; include long operation/error text and variable-width/dynamic inputs.
- Reconcile the status requirement and desktop blueprint to the authorized removal/clipping behavior; document Diagnostics as retained now and deferred for pre-release removal.

## Out of scope

- Removing Diagnostics now; changing its privacy-safe content or affordance beyond preventing status-row wrapping/growth.
- Removing page state, zoom, clean/unsaved state, real operation/error feedback, toolbar behavior, or editor content.
- Changing project/core state, persistence/schema, fixed pagination, source mapping, title-page data, PDF/export, global UI scaling, or unrelated panels.
- New actions/icons, broad status redesign, a new breakpoint without evidence, or an arbitrary footer cap that hides controls instead of clipping overflow.

## Requirements and durable-document impact

- **Requirement:** [`REQ-WRITING-LOOP-006`](../../../requirements/features/screenplay-writing-loop.md:142), especially AC-WRITING-006.1–006.4 and AC-WRITING-006.8. Update only criteria needed to remove the displayed frame/path/initial-placeholder claims and define bounded clipped single-line content; retain truthful operation/error/accessibility and Diagnostics-now behavior.
- **Blueprint:** [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:126). Update projected status fields and narrow layout; preserve status ownership, visible-page freshness, diagnostics privacy, and page/zoom contract.
- Keep fixed screenplay geometry/source-map/PDF contracts unchanged.

## Implementation files

- **Allowed product paths:** [`apps/scriptcraft/src/status_bar.rs`](../../../../apps/scriptcraft/src/status_bar.rs:441) and [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1149), plus focused tests in those app modules.
- **Allowed durable records:** linked requirement and desktop blueprint; this WO; [`status-bar-v2.md`](../status-bar-v2.md); [`execution-log.md`](../execution-log.md).
- **Avoid changing:** toolbar module, core, Cargo manifests/lockfile, font assets, WO-004/validation, earlier v1 work orders/validations, `.DS_Store`, and unrelated files.

## Measurable acceptance and focused regressions

1. Rendered status output contains no `Frames:` item, `Path:` item or `(not saved)` path display, and no startup `In-memory demo project` placeholder. Diagnostics remains openable and its privacy-safe counters remain available.
2. At 1200, breakpoint-adjacent, 480, and 320 logical-point widths, status content and an expanded Diagnostics summary are single-line; no text/icon shape escapes its assigned clip or overlaps page/zoom regions. For unchanged viewport and Diagnostics state, panel height range over frames 2–60 is `≤1.0` logical point. Changing long operation/error strings and diagnostic counters adds no row and changes stable footer height by at most `1.0` point; opening Diagnostics may add only its intentional fixed row.
3. Existing page indicator, zoom slider/presets, keyboard/accessibility behavior, clean/unsaved state, real operation errors, and Diagnostics reachability remain covered. At narrow widths, overflow clips rather than wraps or grows the panel; control hit rectangles remain usable where the viewport affords them.
4. Tests inspect actual renderer shapes/clip rectangles and actual panel geometry, not only projection strings or a hand-built approximation.

## Checks and handoff evidence

- Run focused `cargo test -p scriptcraft status_bar_` and relevant visible-page/zoom/toolbar regressions.
- Run project gates: `cargo fmt --all -- --check`; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`.
- Native visual/input smoke is recommended but not a configured hard-stop; if unavailable, say unavailable and do not claim visual success. Record exact changed paths, all attempts/results, durable-document reconciliation, and Git state.
- **Acceptance test:** Given long/dynamic status content at the stated widths, when the real renderer lays out successive frames, removed items remain absent, retained content clips within its region without wrapping/overlap or dynamic footer growth, Diagnostics remains reachable, and page/zoom/error behavior remains intact.

## Implementation evidence — 2026-10-29

- **Review:** The FAST gross-alignment review passed under host dispatch `01a185e5-73db-7287-a08d-cef519fbfd99`. No separate human design pass is claimed.
- **Implementation:** Removed visible frame/path/not-saved fields and the startup in-memory-demo status placeholder while retaining their underlying application/diagnostic state. Clean/unsaved, operation/error, visible-page, zoom, and Diagnostics presentation remain. Compact and wide layouts use clipped single-line regions; compact zoom controls reserve a bounded responsive width. Diagnostics remains for now and is documented as deferred for pre-release removal.
- **Rendered regressions:** `cargo test -p scriptcraft status_bar_ -- --nocapture` — 15 passed. The renderer-level repeated-frame regression checks 1200 and 480 widths across 60 frames; the dynamic-content test covers 1200, 721, 720, 480, and 320 widths with long/changing status text, page values, and diagnostic counters, inspecting emitted text shapes/clips and panel geometry. `cargo test -p scriptcraft visible_page_` — 4 passed; `cargo test -p scriptcraft zoom_` — 10 passed; `cargo test -p scriptcraft toolbar_` — 28 passed.
- **Workspace gates:** `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`, `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and final `git diff --check` passed. The first Clippy attempt flagged the test-only renderer fixture's argument count; a narrowly scoped lint allowance was added, then Clippy passed.
- **Native smoke:** Native visual/input smoke was not performed; no native visual-success claim is made.
- **Dispatch/review boundary:** At the initial implementation handoff, the implementer's host dispatch ID was pending because none had been supplied; the user later supplied and reconciled actual host dispatch ID `01a185e7-91a8-74b8-ae3b-570218150a31` in this WO's metadata, the v2 plan, and execution log. No internal session ID is substituted. Implementation is complete for sequential handoff to WO-006, but this WO remains `in-review` pending the user's one aggregate validation after all four WOs. No independent validation is claimed.

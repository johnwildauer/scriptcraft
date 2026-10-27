# Validation — WO-002 Toolbar Chrome Color

## Validation metadata

- **Validation type:** `implementation alignment and anti-rogue validation`
- **Status:** `FAIL`
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `debug-fresh-context-toolbar-final-fit-WO-002-validation-2026-10-27T0437Z`
- **Date:** 2026-10-27
- **Work order:** [`WO-002-toolbar-chrome-color.md`](../work-orders/WO-002-toolbar-chrome-color.md:1)
- **Initiative:** [`toolbar-final-fit-v1.md`](../toolbar-final-fit-v1.md:1)
- **Fresh-context statement:** This validation was performed as a separate fresh-context validator session. I did not implement WO-002, did not rely on implementer conversational memory, did not validate unrelated work, and did not mark WO-002 complete.

## Scope and comparison basis

This validation covers only the toolbar chrome color source/application, deterministic style tests, the documented native OS title-bar boundary, preservation of geometry and global theme behavior, durable-document impact, and the required Cargo/diff gates. It does not revalidate WO-001 geometry/padding, core behavior, persistence, exports, editor layout, or unrelated initiative work.

Inspected:

- Validation and lifecycle rules in [`05-validation.md`](../../../process/05-validation.md:1) and [`00-overview.md`](../../../process/00-overview.md:1).
- Project commands and gate policy in [`project-manifest.md`](../../../project-manifest.md:73).
- Initiative scope and design warning DR-W-002 in [`toolbar-final-fit-v1.md`](../toolbar-final-fit-v1.md:24) and [`design-review.md`](../reviews/design-review.md:19).
- Authorized scope, durable-document classification, implementation report, and required checks in [`WO-002-toolbar-chrome-color.md`](../work-orders/WO-002-toolbar-chrome-color.md:13).
- WO-001 dependency evidence in [`WO-001-toolbar-fit-and-vertical-padding.md`](../work-orders/WO-001-toolbar-fit-and-vertical-padding.md:59) and [`validation-WO-001.md`](validation-WO-001.md:1).
- Dispatch, changed-file, and implementation evidence in [`execution-log.md`](../execution-log.md:74).
- Comparable working-tree diff for [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1) and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:58), with cumulative WO-001 changes distinguished from the WO-002 boundary.

## Diagnostic hypotheses and validation evidence

The possible sources of an invalid result were: (1) the helper was never added, (2) the helper exists under a different name or module, (3) the toolbar panel never applies the frame, (4) the style test references an unavailable/private API, (5) the frame changes geometry defaults, (6) the implementation mutates global visuals, or (7) the native title-bar limitation is overstated as pixel equality.

The two most likely sources were the missing helper definitions and the missing application site. Repository search found only the test references to [`toolbar_chrome_color()`](../../../../apps/scriptcraft/src/main.rs:1455) and [`toolbar_frame()`](../../../../apps/scriptcraft/src/main.rs:1456); no definitions were present. The update path still calls [`TopBottomPanel::top("toolbar").show()`](../../../../apps/scriptcraft/src/main.rs:1371) directly and does not call a frame helper. The gate logs below validate these observations without adding production logging or changing production code.

## Findings by severity

### Blockers

#### F-001 — Required chrome source/application is absent and deterministic tests do not compile

- **Severity:** hard-stop
- **Evidence:** [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1455) calls [`toolbar_chrome_color()`](../../../../apps/scriptcraft/src/main.rs:1455) and [`toolbar_frame()`](../../../../apps/scriptcraft/src/main.rs:1456), but repository search found no definitions. The production update path at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1371) still renders the toolbar with `TopBottomPanel::top("toolbar").show(ctx, ...)` and does not apply an explicit frame or `panel_fill` source.
- **Gate evidence:** `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` both fail with `E0425: cannot find function toolbar_chrome_color` and `E0425: cannot find function toolbar_frame`. Therefore the claimed deterministic style coverage cannot run, and the acceptance criterion that the toolbar use the documented app-chrome source is not met.
- **Recommendation:** Implement the missing focused helper(s), apply the selected egui `Visuals::panel_fill` source to the toolbar frame, and ensure the tests compile and pass. Keep the change within WO-002; do not alter geometry or global style state.
- **Required next state:** The orchestrator must keep WO-002 `in-review`, create a letter-suffixed fix work order within WO-002's boundary, and dispatch a new implementer session followed by a fresh validation. WO-002 must not be marked complete from this result.

### Warnings

#### W-001 — Native OS title-bar pixel comparison remains unavailable

- **Severity:** warning
- **Evidence:** The work order and blueprint correctly state that eframe's default native decorations own the title bar and expose no sampleable title-bar color in this setup ([`WO-002-toolbar-chrome-color.md`](../work-orders/WO-002-toolbar-chrome-color.md:58), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:88)). No native visual/pixel equality was observed or claimed.
- **Recommendation:** Preserve the boundary statement after the fix. Deterministic equality to egui's exposed application-surface source is acceptable evidence; it must not be described as native OS title-bar pixel equality.
- **Required next state:** Follow-up/documentation note only after the hard-stop implementation failure is fixed; no native visual claim may be added without platform evidence.

## Alignment notes

- **Geometry preservation:** The intended test [`toolbar_frame_preserves_top_panel_geometry_defaults()`](../../../../apps/scriptcraft/src/main.rs:1461) compares frame margins, rounding, shadow, and stroke with `Frame::side_top_panel`, which is aligned with the authorized preservation boundary. It cannot currently provide evidence because the helper is undefined. The cumulative diff also contains WO-001 geometry changes, but no WO-002-specific geometry mutation beyond the unimplemented frame path was observed.
- **Global theme behavior:** The intended style test checks that the context's `panel_fill` remains unchanged. No global style mutation was observed in the available diff. This remains unverified at runtime because the test target fails to compile.
- **Durable-document impact:** The blueprint update at [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:88) is the authorized durable-document update for a presentation-only change, and requirements remain classified `verify unchanged` in [`WO-002-toolbar-chrome-color.md`](../work-orders/WO-002-toolbar-chrome-color.md:48). The requirement diff present in the working tree belongs to cumulative WO-001 wording and is not treated as WO-002 scope. The classification is not plainly contradicted, but the implementation evidence is incomplete until the source/application exists.
- **Diff/rogue review:** The changed production area and blueprint are generally the authorized files, and no deceptive, backdoor, malicious, nefarious, or unrelated subsystem behavior was observed. The missing implementation is a genuine unmet required criterion, not a scope or style preference.

## Validator gate results

| Check | Exact result | Outcome |
|---|---|---|
| `cargo fmt --all -- --check` | Exit 0; no formatter diagnostics | PASS |
| `cargo check --workspace` | Exit 0; non-test targets compiled | PASS |
| `cargo test --workspace` | Exit 101; test target failed to compile with three `E0425` missing-function errors for `toolbar_chrome_color`/`toolbar_frame` | FAIL |
| `cargo build --workspace` | Exit 0; dev profile finished | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | Exit 101; same three missing-function errors in the test target | FAIL |
| `git diff --check` | Exit 0; no whitespace errors | PASS |

No native visual smoke or pixel comparison was available. No native visual success or native title-bar equality is claimed.

## Verdict and required next state

- **Result:** `FAIL`.
- **Why:** The submitted WO-002 evidence describes helpers and application that are absent from the checked-out source. The production toolbar still uses the unframed panel path, and the new deterministic tests reference undefined functions. Required workspace tests and all-target Clippy therefore fail. This is a hard-stop unmet acceptance criterion and required gate, not a design preference.
- **Required next state:** Keep [`WO-002-toolbar-chrome-color.md`](../work-orders/WO-002-toolbar-chrome-color.md:1) `in-review`; do not mark it complete. The orchestrator should create a scoped fix work order, preserve this validation as the failure evidence, and dispatch a new implementation/validation session pair. The native OS title-bar limitation remains an honest warning rather than a reason to claim pixel equality.

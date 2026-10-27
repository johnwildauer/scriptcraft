# Validation: WO-001 — Transparent Native Viewport

## Validation metadata

- **Validation type:** `fresh-context implementation validation`
- **Validator session/agent ID:** `validation-WO-001-toolbar-viewport-hover-2026-10-27T0538Z`
- **Date:** 2026-10-27
- **Work order:** [`WO-001-transparent-native-viewport.md`](../work-orders/WO-001-transparent-native-viewport.md:1)
- **Initiative:** [`toolbar-viewport-hover-v1.md`](../toolbar-viewport-hover-v1.md:1)
- **Work-order status at validation:** `in-review`
- **Session separation exception:** none

## Comparison basis

Fresh-context review used the installed validation process, project policy, initiative plan, design review, work-order evidence, execution log, the comparable tracked diff, and the current source. The reviewed implementation surface is limited to [`native_options()`](../../../../apps/scriptcraft/src/main.rs:1461), its [`main()`](../../../../apps/scriptcraft/src/main.rs:1474) caller, the options test, and the authorized desktop blueprint update in [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:90).

WO-002 and WO-003 were explicitly excluded from this validation. No hover-region or toolbar-edge behavior was assessed.

## Findings

### PASS — installed eframe viewport API mapping

- **Severity:** pass
- **Evidence:** The app depends on eframe `0.29` in [`Cargo.toml`](../../../../apps/scriptcraft/Cargo.toml:8). [`native_options()`](../../../../apps/scriptcraft/src/main.rs:1461) builds the installed `ViewportBuilder` API with `with_title_shown(false)`, `with_titlebar_shown(false)`, and `with_fullsize_content_view(true)` at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1463). The options test asserts the resulting `title_shown`, `titlebar_shown`, and `fullsize_content_view` fields at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1493).
- **Assessment:** This is the approved installed-API equivalent of the requested hidden-title, transparent-titlebar, and full-size-content configuration. The implementation is aligned with WO-001 and does not introduce unrelated production behavior.
- **Required next state:** Retain the implementation for the next initiative work order; no fix work order is required.

### PASS — native runner wiring and startup preservation by available evidence

- **Severity:** pass
- **Evidence:** [`main()`](../../../../apps/scriptcraft/src/main.rs:1474) obtains the configured options and passes them to `eframe::run_native` at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1476). The required check, test, and build gates all exited 0, including the options test.
- **Assessment:** Compilation, workspace tests, and runner construction support preserved startup behavior within the available automated evidence.
- **Required next state:** Keep WO-001 `in-review` until the orchestrator applies the later lifecycle transition; this validation does not mark it complete.

### PASS — options test and documentation

- **Severity:** pass
- **Evidence:** [`native_options_configure_the_requested_viewport_presentation()`](../../../../apps/scriptcraft/src/main.rs:1494) verifies all three resulting fields without opening a native window. The authorized update to [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:90) records the eframe 0.29 mapping, macOS-specific behavior, platform-controlled decorations, and the prohibition on pixel-level cross-platform titlebar claims.
- **Assessment:** The test and durable-document update satisfy the work-order obligations and accurately preserve the platform boundary.
- **Required next state:** No documentation correction is required.

### WARNING — native visual smoke unavailable

- **Severity:** warning (non-blocking under current project policy)
- **Evidence:** The implementation evidence and execution log state that native visual smoke was unavailable. The project manifest configures no required manual/external gate at [`project-manifest.md`](../../../project-manifest.md:88), and the blueprint explicitly limits these settings to native presentation hints at [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:90).
- **Assessment:** The viewport fields are validated as configuration values only. This validation does **not** claim that the native title is visually hidden, that the titlebar is visually transparent, or that content visibly reaches the titlebar on a running platform. Unsupported-platform behavior remains platform-controlled.
- **Required next state:** Preserve the limitation in the work-order and validation evidence; do not convert it into a native visual success claim.

### PASS — required Cargo and diff gates

- **Severity:** pass
- **Evidence:** A fresh chained run on 2026-10-27 completed with exit code 0 for `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`, `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check`. The workspace test suite passed, including the named viewport-options test; no whitespace errors were reported. The comparable diff contains only the authorized app entry-point change and desktop blueprint documentation change, with initiative evidence files untracked as expected.
- **Assessment:** All required project-policy gates available in this checkout passed. No clearly unrelated, deceptive, backdoor, malicious, or nefarious behavior was found.
- **Required next state:** No gate remediation is required.

## Result

- **Outcome:** `pass with warning`
- **Hard-stop findings:** none
- **Warnings/notes:** Native visual smoke was unavailable; no native visual success is claimed.
- **WO-001 disposition:** Remains `in-review`; this validation intentionally does not mark the work order `complete`.
- **Scope boundary:** WO-002 and WO-003 remain pending and were not implemented or validated here.

## Required next state

The orchestrator should record this validation in the initiative execution log, keep the documented native-visual limitation, and only perform the next dependency-ordered dispatch after applying the normal lifecycle transition. WO-002 and WO-003 require their own implementation and fresh validation sessions; they are outside this result.

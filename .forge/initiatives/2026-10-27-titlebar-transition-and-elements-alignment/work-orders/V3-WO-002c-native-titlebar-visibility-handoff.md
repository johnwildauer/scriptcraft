# Work Order: V3-WO-002c — Native Titlebar Visibility Handoff

## Metadata

- **Status:** `plan`
- **Assignee:** Fresh desktop startup-handoff implementer
- **Phase/order:** Second letter-suffixed fix for [`V3-WO-002`](V3-WO-002-stable-titlebar-transition-metrics.md:1), following [`V3-WO-002a`](V3-WO-002a-native-titlebar-scaling-diagnostic.md:1) and [`V3-WO-002b`](V3-WO-002b-native-titlebar-mode-wiring.md:1)
- **Initiative:** [`titlebar-transition-and-elements-alignment-v3.md`](../titlebar-transition-and-elements-alignment-v3.md:1)
- **Original work order:** [`V3-WO-002-stable-titlebar-transition-metrics.md`](V3-WO-002-stable-titlebar-transition-metrics.md:1)
- **Failed parents:** [`V3-WO-002a-native-titlebar-scaling-diagnostic.md`](V3-WO-002a-native-titlebar-scaling-diagnostic.md:1), [`V3-WO-002b-native-titlebar-mode-wiring.md`](V3-WO-002b-native-titlebar-mode-wiring.md:1)
- **Failure record:** [`execution-log.md`](../execution-log.md:1)
- **Validation:** create `validation-V3-WO-002c.md` only in a separate fresh validation session

## Outcome

Correct and prove the complete startup handoff for the native macOS titlebar diagnostic. The selected one-line mode must reach the exact installed eframe viewport options, those same options must reach [`eframe::run_native()`](../../../../apps/scriptcraft/src/main.rs:2588), and no later viewport/native-options or custom title-strip path may hide or visually mask native chrome. This is a wiring correction, not a final titlebar redesign or a native-animation claim.

## Confirmed failure

After [`V3-WO-002b-native-titlebar-mode-wiring.md`](V3-WO-002b-native-titlebar-mode-wiring.md:1), the user rebuilt and launched the supposed native diagnostic mode, but the native macOS titlebar remained hidden/transparent. This is a **confirmed mode-wiring failure**: not unavailable display evidence and not user error. The local tuple and deterministic helper tests did not prove runtime startup provenance.

The parent visual gate therefore remains failed. Do not claim native animation or native scaling results until the user verifies visible native titlebar buttons/titlebar in the corrected build.

[`05-validation.md`](../../../process/05-validation.md:95) and [`00-overview.md`](../../../process/00-overview.md:112) require this letter-suffixed fix rather than reopening the parent. The implementer must consume the failed-parent evidence, current source/diff, V3 plan/review, and V3-WO-001b preservation evidence.

## In scope: end-to-end startup audit and correction

1. **Build and mode provenance.** Verify that the tested clean build actually changes [`TITLEBAR_PRESENTATION_MODE`](../../../../apps/scriptcraft/src/main.rs:49) to [`TitlebarPresentationMode::NativeMacOsDiagnostic`](../../../../apps/scriptcraft/src/main.rs:55) when native mode is selected. Add the smallest deterministic startup label/log or build/handoff diagnostic needed to distinguish the tested native binary from the default custom binary. Selection requires rebuild/relaunch and is not persisted. Keep [`TitlebarPresentationMode::CustomHidden`](../../../../apps/scriptcraft/src/main.rs:54) as default.

2. **Exact startup handoff.** Trace [`selected_titlebar_presentation_mode()`](../../../../apps/scriptcraft/src/main.rs:74) into [`startup_presentation()`](../../../../apps/scriptcraft/src/main.rs:2599). Ensure that function returns the selected mode and options constructed for that same mode. Ensure [`main()`](../../../../apps/scriptcraft/src/main.rs:2582) passes those exact returned options to [`eframe::run_native()`](../../../../apps/scriptcraft/src/main.rs:2588), with no stale helper, alternate runner, shadowed options, or test-only substitute. Add a deterministic handoff seam/test comparing selected mode, returned options, and runner-bound options.

3. **No later overwrite.** Audit [`native_options_for_mode()`](../../../../apps/scriptcraft/src/main.rs:2563), [`native_options()`](../../../../apps/scriptcraft/src/main.rs:2578), [`startup_presentation()`](../../../../apps/scriptcraft/src/main.rs:2599), app creation, frame setup, viewport commands, and any other native-options path. Prove that no later path overwrites title visibility, titlebar visibility, decorations, or full-size-content settings before the window is shown.

4. **Installed API correctness.** Verify the resolved eframe/egui 0.29.1 APIs rather than guessing another version: [`ViewportBuilder::with_title_shown()`](../../../../.cargo/registry/src/index.crates.io-1949da5c6b5b557f/egui-0.29.1/src/viewport.rs:1), [`ViewportBuilder::with_titlebar_shown()`](../../../../.cargo/registry/src/index.crates.io-1949da5c6b5b557f/egui-0.29.1/src/viewport.rs:1), [`ViewportBuilder::with_fullsize_content_view()`](../../../../.cargo/registry/src/index.crates.io-1949da5c6b5b557f/egui-0.29.1/src/viewport.rs:1), and decorations. Native diagnostic values must be title shown `true`, titlebar shown `true`, decorations `true`, full-size content `false` unless the installed API audit proves the compatible native equivalent. Custom values remain `(false, false, true)` with decorations `true`.

5. **Renderer and policy boundary.** In native mode [`ScriptCraftApp::update()`](../../../../apps/scriptcraft/src/main.rs:2466) must not paint the custom title strip or centered `ScriptCraft`, send [`title_strip_viewport_command()`](../../../../apps/scriptcraft/src/main.rs:2411), or create a transparent/full-size app-owned surface over native chrome. Native mode must bypass complete [`PresentationTransitionPolicy`](../../../../apps/scriptcraft/src/main.rs:2004) request/capture/hold/release and settling-repaint behavior. Custom mode retains title strip, title text, known-state command behavior, and app-owned policy.

6. **Direct visual gate.** Clean-build/relaunch both modes and record command/build identity, selected mode, and date/session. Native mode must visibly show native macOS titlebar/title and red/yellow/green buttons, with the egui toolbar usable below/within the native content boundary and no duplicate custom strip. Only after this gate passes may the user report maximize/restore animation behavior.

## Preservation contract

- Preserve V3-WO-001b’s black-bar/clear-color result, including [`ScriptCraftApp::clear_color()`](../../../../apps/scriptcraft/src/main.rs:2462), shared chrome fill, and [`TOOLBAR_BOTTOM_PADDING`](../../../../apps/scriptcraft/src/main.rs:37). No overlay, padding removal, central-fill change, or toolbar compression may conceal native chrome.
- Preserve [`TOOLBAR_ELEMENTS_WIDTH`](../../../../apps/scriptcraft/src/main.rs:28) at `128.0`, toolbar allocation/right-edge/padding, toolbar/Elements/popup/cursor/input/IME/accessibility/semantic/page contracts, and app-owned transition-policy boundaries.
- Do not make native mode default, persist it, modify validation artifacts, or alter requirements/blueprints/core crates/closed work orders.

## Likely files and boundaries

- Primary source/tests: [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1), limited to mode provenance, startup/options handoff, installed option values, mode-aware renderer/title-strip branches, diagnostic evidence, and focused tests.
- Dependency/API evidence: [`apps/scriptcraft/Cargo.toml`](../../../../apps/scriptcraft/Cargo.toml:8), [`Cargo.lock`](../../../../Cargo.lock:875), and resolved eframe/egui 0.29.1 source under [`.cargo/registry`](../../../../.cargo/registry/src/index.crates.io-1949da5c6b5b557f/egui-0.29.1/src/viewport.rs:1).
- Evidence documents permitted: this fix WO and [`execution-log.md`](../execution-log.md:1). Do not modify [`validation-V3-WO-002b.md`](../validations/validation-V3-WO-002b.md:1) or any validation artifact.

## Deterministic tests and required checks

Tests must prove custom-default/native-reversible selection; exact custom/native option tuples; selected-mode-to-`startup_presentation()` provenance; exact runner handoff; native title-strip suppression and policy bypass; custom title-strip/policy preservation; unknown maximize safety; no global egui zoom/pixels-per-point mutation; and unchanged toolbar/Elements/popup/cursor/input contracts.

Run after a clean rebuild and record exact selectors/results:

```text
cargo clean
cargo fmt --all -- --check
cargo test -p scriptcraft <focused startup/mode/options/handoff tests>
cargo check --workspace
cargo test --workspace
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

Automated tests prove wiring/preservation only; they cannot prove visible macOS chrome.

## Pass / fail / blocked

**Pass** requires unambiguous native build provenance; selected mode flowing through [`startup_presentation()`](../../../../apps/scriptcraft/src/main.rs:2599) to the exact [`eframe::run_native()`](../../../../apps/scriptcraft/src/main.rs:2588) options without overwrite; correct installed values; visible native titlebar/buttons; no custom masking; usable toolbar; native policy bypass; preserved custom/default/black-bar/toolbar/Elements/cursor/popup/input contracts; clean checks; and the user’s two-mode record. Native animation is recorded only after visible chrome passes.

**Fail** if provenance or exact handoff is unproven, any path overwrites options, installed values are wrong, titlebar/buttons remain hidden/transparent, custom strip masks native chrome, toolbar is unusable, native policy remains active, or a preservation contract regresses.

**Blocked** only after both modes are clean-built/launched, handoff is proven, and a specific external platform/compositor limitation prevents meaningful observation. “No display available,” “native evidence unavailable,” or an unverified launch is not a pass.

## Durable-document classification

- **Requirements:** `verify unchanged` — [`REQ-WRITING-LOOP-005`](../../../requirements/features/screenplay-writing-loop.md:90), [`AC-WRITING-005.5`](../../../requirements/features/screenplay-writing-loop.md:115), [`AC-WRITING-005.6`](../../../requirements/features/screenplay-writing-loop.md:131), and [`AC-WRITING-005.7`](../../../requirements/features/screenplay-writing-loop.md:137) remain the boundary; this diagnostic does not approve a final native-titlebar contract.
- **Blueprint:** `verify unchanged` — [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:83) and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:105) already describe the native/presentation boundary.
- **Rationale:** this is an implementation-provenance correction inside a reversible diagnostic; it creates no durable product promise.
- **Authorized changes:** this fix WO and execution log only. No source or validation artifact is modified by this planning action.

## Fresh-session requirements and next state

Dispatch a new implementer and then a separate fresh validator. Keep the initiative in planning/review pending approval; this artifact does not authorize implementation. The implementer leaves this WO `in-review` with source/diff evidence, clean checks, and the direct two-mode visual record. The validator owns pass/fail and must inspect startup provenance, exact options, overwrite paths, renderer boundary, policy bypass, preservation contracts, user gate, and durable classification. Do not create [`validation-V3-WO-002c.md`](../validations/validation-V3-WO-002c.md:1) during planning.

**Next state:** `plan` — review and approval precede implementation; completion requires fresh validation and an unmistakable user-confirmed native macOS titlebar/titlebar-button gate. No native animation result is claimed before that gate.
# Work Order: V3-WO-002c — Native Titlebar Visibility Handoff

## Metadata

- **Status:** `in-review`
- **Assignee:** Fresh desktop startup-handoff implementer
- **Phase/order:** Second letter-suffixed fix for V3-WO-002, following V3-WO-002a and V3-WO-002b
- **Initiative:** [`titlebar-transition-and-elements-alignment-v3.md`](../titlebar-transition-and-elements-alignment-v3.md:1)
- **Original work order:** [`V3-WO-002-stable-titlebar-transition-metrics.md`](V3-WO-002-stable-titlebar-transition-metrics.md:1)
- **Failed parents:** V3-WO-002a and V3-WO-002b
- **Implementer session/agent ID:** `implementer-fresh-v3-wo002c-native-titlebar-handoff-2026-10-27T2047Z`
- **Session exception:** none
- **Validation:** create `validation-V3-WO-002c.md` only in a separate fresh validation session; none was created here

## Outcome and boundary

Correct and prove the tested-build native macOS titlebar startup handoff. The selected `TITLEBAR_PRESENTATION_MODE` must reach one identity-bearing startup presentation and the exact `eframe::NativeOptions` object passed to `eframe::run_native()`. Native mode must request visible native chrome and non-full-size content, suppress the custom title strip/maximize ownership, bypass the app-owned transition policy, and retain a usable toolbar. Custom mode, the V3-WO-001b black-bar/clear-color/padding correction, and all toolbar/Elements/cursor/popup/input contracts remain unchanged. This is not a transition-policy redesign or a native animation claim.

## Implementation evidence — 2026-10-27

- **Branch/status before changes:** Confirmed `dev/2026-10-27-titlebar-transition-and-elements-alignment`; cumulative V3 source and Forge evidence were preserved. The V3-WO-001b clear-color owner, restored `TOOLBAR_BOTTOM_PADDING = 2.0`, toolbar allocation, and `TOOLBAR_ELEMENTS_WIDTH = 128.0` were not changed. No reset, checkout, commit, validation artifact, or destructive Git action was used.
- **Exact diagnosis:** V3-WO-002b still had two independently selected startup paths: `startup_presentation()` selected a mode, then called `native_options()`, which selected the compile-time constant again. The tuple helper was correct, but that seam did not prove that the selected mode and the exact object supplied to `eframe::run_native()` were one handoff. The installed API audit also showed that the viewport title was not explicitly set; the runner title alone was not an auditable viewport-option provenance field.
- **Correction:** [`StartupPresentation`](../../../../apps/scriptcraft/src/main.rs:2579) now carries the selected mode and exact `eframe::NativeOptions`. [`startup_presentation_for_mode()`](../../../../apps/scriptcraft/src/main.rs:2584) constructs both from one mode argument; [`startup_presentation()`](../../../../apps/scriptcraft/src/main.rs:2591) selects the compile-time constant once; and [`main()`](../../../../apps/scriptcraft/src/main.rs:2597) destructures and moves that exact options object directly into [`eframe::run_native()`](../../../../apps/scriptcraft/src/main.rs:2601). The independently selecting `native_options()` path was removed. Both modes explicitly set `.with_title("ScriptCraft")`; native is `(title_shown=true, titlebar_shown=true, decorations=true, fullsize_content_view=false)`, while custom remains `(false, false, true)` with decorations true.
- **No-overwrite audit:** There is one `eframe::run_native()` call and one startup options construction path. No later application path writes title/titlebar/decorations/full-size-content settings. App update branches custom title-strip painting, custom maximize command, and custom transition-policy repainting by the selected mode; native mode retains the toolbar as app content and bypasses custom title-strip/maximize/policy ownership. No global egui zoom or pixels-per-point mutation was added.
- **Installed API evidence:** Resolved `eframe/egui 0.29.1` source confirms `ViewportBuilder::with_title`, `with_decorations`, `with_fullsize_content_view`, `with_title_shown`, and `with_titlebar_shown` set the corresponding `ViewportBuilder` fields. The native tuple uses the installed macOS semantics.
- **Deterministic evidence:** [`native_options_are_exact_and_reversible_for_both_modes()`](../../../../apps/scriptcraft/src/main.rs:2631) asserts both exact tuples, explicit title, per-mode handoff identity, and selected-mode provenance. Existing titlebar surface/policy, command-safety, black-bar/clear-color/padding, toolbar/Elements, cursor, popup, input, semantic, and page tests remain unchanged and passing.
- **Checks:** `cargo clean && cargo fmt --all -- --check && cargo test -p scriptcraft native_options_are_exact_and_reversible_for_both_modes && cargo test -p scriptcraft titlebar && cargo test -p scriptcraft title_strip_viewport_toggle_is_safe_for_known_and_unknown_state && cargo check --workspace && cargo test --workspace && cargo build --workspace && cargo clippy --workspace --all-targets -- -D warnings && git diff --check` passed with exit 0. The clean workspace test/build/check/clippy chain passed.
- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1), this work order, and [`execution-log.md`](../execution-log.md:1). No validation artifact, requirement, blueprint, core crate, closed work order, black-bar correction, or unrelated transition policy was changed.
- **User gate / next state:** `in-review`. No visual success is claimed. The user must switch the one-line constant to `TitlebarPresentationMode::NativeMacOsDiagnostic`, clean-rebuild/relaunch, verify unmistakable native macOS titlebar visibility/buttons, usable toolbar, and no duplicate custom strip, then compare native/custom maximize/restore behavior. Validation must not be dispatched until that result is reported.

## Durable-document classification

- **Requirements:** `verify unchanged` — the reversible diagnostic does not change the writing-loop, toolbar, Elements, cursor, popup, or input contract.
- **Blueprint:** `verify unchanged` — existing native presentation and app-chrome boundaries remain accurate; no final native-titlebar product decision is made.

## Next state

`in-review`; a fresh validator remains required after the user reports the direct clean-build native/custom visual gate. Do not create `validation-V3-WO-002c.md` or mark this work order complete in this session.

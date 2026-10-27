# Work Order: V3-WO-002d — Native Titlebar Direct Configuration

## Metadata

- **Status:** `plan`
- **Assignee:** Fresh desktop native-presentation debug implementer
- **Phase/order:** Third letter-suffixed fix for [`V3-WO-002`](V3-WO-002-stable-titlebar-transition-metrics.md:1), after failed [`V3-WO-002a`](V3-WO-002a-native-titlebar-scaling-diagnostic.md:1), [`V3-WO-002b`](V3-WO-002b-native-titlebar-mode-wiring.md:1), and [`V3-WO-002c`](V3-WO-002c-native-titlebar-visibility-handoff.md:1)
- **Initiative:** [`titlebar-transition-and-elements-alignment-v3.md`](../titlebar-transition-and-elements-alignment-v3.md:1)
- **Original work order:** [`V3-WO-002-stable-titlebar-transition-metrics.md`](V3-WO-002-stable-titlebar-transition-metrics.md:1)
- **Failed parents:** [`V3-WO-002-stable-titlebar-transition-metrics.md`](V3-WO-002-stable-titlebar-transition-metrics.md:1), [`V3-WO-002a-native-titlebar-scaling-diagnostic.md`](V3-WO-002a-native-titlebar-scaling-diagnostic.md:1), [`V3-WO-002b-native-titlebar-mode-wiring.md`](V3-WO-002b-native-titlebar-mode-wiring.md:1), [`V3-WO-002c-native-titlebar-visibility-handoff.md`](V3-WO-002c-native-titlebar-visibility-handoff.md:1)
- **Failure evidence:** [`execution-log.md`](../execution-log.md:387)
- **Validation:** create [`validation-V3-WO-002d.md`](../validations/validation-V3-WO-002d.md:1) only in a separate fresh validation session

## Outcome

Determine why the supposedly native macOS viewport still renders with hidden or transparent native titlebar chrome after the direct startup-handoff correction, then make the diagnostic unambiguous. This is a bounded configuration/ownership diagnosis, not approval of native chrome as the product default and not a promise to control native animation internals.

## Confirmed third failure

After [`V3-WO-002c-native-titlebar-visibility-handoff.md`](V3-WO-002c-native-titlebar-visibility-handoff.md:1) and switching the selected mode to `NativeMacOsDiagnostic`, the user again observed that the native macOS titlebar and red/yellow/green buttons were hidden or transparent. This is a **confirmed failure of the diagnostic**. It is not unavailable display evidence, not a missing display, and not user error. The prior exact tuple, selected-mode handoff tests, clean rebuild, and local no-overwrite audit are insufficient because the running window still does not visibly demonstrate native chrome.

The failure remains attached to the chain [`V3-WO-002`](V3-WO-002-stable-titlebar-transition-metrics.md:1) → [`V3-WO-002a`](V3-WO-002a-native-titlebar-scaling-diagnostic.md:1) → [`V3-WO-002b`](V3-WO-002b-native-titlebar-mode-wiring.md:1) → [`V3-WO-002c`](V3-WO-002c-native-titlebar-visibility-handoff.md:1). Do not dispatch final [`validation-V3-WO-002.md`](../validations/validation-V3-WO-002.md:1) or claim animation/scaling evidence until native chrome is visibly proven.

## In scope

1. **Audit every native option and override.** Inspect resolved eframe 0.29.1/egui-winit/winit source and the complete startup path for interactions among `with_title_shown`, `with_titlebar_shown`, `with_decorations`, `with_fullsize_content_view`, `with_title`, transparent-titlebar behavior, `decorations`, and platform-specific viewport attributes. Search for later construction, normalization, merge, command, or platform-default paths that override or reinterpret values before the macOS window is shown. Verify the actual winit version from [`Cargo.lock`](../../../../Cargo.lock:875).

2. **Audit visual ownership and masking.** Trace whether [`ScriptCraftApp::update()`](../../../../apps/scriptcraft/src/main.rs:2466), [`TopBottomPanel`](../../../../apps/scriptcraft/src/main.rs:2478), [`title_strip()`](../../../../apps/scriptcraft/src/main.rs:2419), [`title_strip_frame()`](../../../../apps/scriptcraft/src/main.rs:2342), [`central_panel_frame()`](../../../../apps/scriptcraft/src/main.rs:2280), [`chrome_clear_color()`](../../../../apps/scriptcraft/src/main.rs:2290), toolbar fill, clear color, clips, or content allocation paints over, covers, or visually mimics native chrome in native mode. Check whether a zero-height or transparent app-owned strip still occupies the native boundary. The toolbar is app content, not evidence of native chrome.

3. **Make native mode unmistakable if needed.** Add the smallest reversible diagnostic in [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1): direct native title/toolbar/window configuration that cannot be confused with the custom strip, plus a bounded startup diagnostic visible in the built binary or startup output. It must state selected mode and effective title, titlebar, decorations, full-size content, transparency/platform attributes, and other applied native values. Do not rely only on a unit-test helper or the runner title argument.

4. **Keep modes separable.** `CustomHidden` remains the compile-time/default product mode. Native mode must not paint a custom title strip, centered placeholder title, transparent overlay, or competing maximize hit region. Custom mode retains its existing title-strip and app-owned transition-policy behavior. Do not persist the diagnostic.

5. **Preserve contracts.** Preserve [`ScriptCraftApp::clear_color()`](../../../../apps/scriptcraft/src/main.rs:2462), shared chrome fill, [`TOOLBAR_BOTTOM_PADDING`](../../../../apps/scriptcraft/src/main.rs:37), [`TOOLBAR_ELEMENTS_WIDTH`](../../../../apps/scriptcraft/src/main.rs:28) at `128.0`, toolbar allocation/right edge, Elements/popup/cursor/input/IME/accessibility/semantic/page behavior, and custom transition-policy boundaries. No blind overlay, padding removal, central-fill substitution, or toolbar compression may be used as a titlebar remedy.

## Out of scope

- Rewriting the app-owned transition algorithm; native animation progress/interpolation/control; global egui scaling; desktop/work-area polling; `has_requested_resize`; unsafe hooks; or a final native/custom product decision.
- Broad theme/layout, toolbar, Elements, semantic/core, persistence/export, closed v1/v2, requirements, blueprint, or validation-artifact changes.
- Passing because a window launched, options look correct in a helper, or native evidence becomes unavailable. The visible native-titlebar gate is mandatory.

## Exact source/API boundary

- Primary source/tests: [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1), especially [`TITLEBAR_PRESENTATION_MODE`](../../../../apps/scriptcraft/src/main.rs:49), [`TitlebarPresentationMode`](../../../../apps/scriptcraft/src/main.rs:52), [`selected_titlebar_presentation_mode()`](../../../../apps/scriptcraft/src/main.rs:74), [`native_options_for_mode()`](../../../../apps/scriptcraft/src/main.rs:2563), [`startup_presentation()`](../../../../apps/scriptcraft/src/main.rs:2591), [`main()`](../../../../apps/scriptcraft/src/main.rs:2595), [`ScriptCraftApp::update()`](../../../../apps/scriptcraft/src/main.rs:2466), [`title_strip()`](../../../../apps/scriptcraft/src/main.rs:2419), [`chrome_frame()`](../../../../apps/scriptcraft/src/main.rs:2315), [`title_strip_frame()`](../../../../apps/scriptcraft/src/main.rs:2342), [`central_panel_frame()`](../../../../apps/scriptcraft/src/main.rs:2280), and [`ScriptCraftApp::clear_color()`](../../../../apps/scriptcraft/src/main.rs:2462).
- Dependency/API evidence: [`apps/scriptcraft/Cargo.toml`](../../../../apps/scriptcraft/Cargo.toml:8), [`Cargo.lock`](../../../../Cargo.lock:875), and resolved eframe/egui 0.29.1, egui-winit, and winit sources under the local Cargo registry. Record actual resolved paths/version if the registry hash differs.
- Permitted evidence changes: this WO and [`execution-log.md`](../execution-log.md:1) only. Do not modify [`validation-V3-WO-002c.md`](../validations/validation-V3-WO-002c.md:1) or any validation artifact.

## Deterministic tests and required checks

Tests must prove custom-default/native-reversible selection; exact effective custom/native values for title, title/titlebar visibility, decorations, full-size content, transparency/platform attributes, and diagnostic label; selected mode and exact options as one handoff to the sole [`eframe::run_native()`](../../../../apps/scriptcraft/src/main.rs:2601); no later overwrite or native-mode custom paint/overlay/maximize ownership; custom policy/title-strip preservation; no global scale mutation; and unchanged black-bar, clear-color/padding, toolbar, Elements `128.0`, popup, cursor, input, and semantic contracts.

After a clean rebuild, record exact results for `cargo clean`, `cargo fmt --all -- --check`, focused direct-configuration/provenance/ownership tests, `cargo check --workspace`, `cargo test --workspace`, `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check`. Automated tests prove wiring and preservation only; they cannot prove visible macOS chrome.

## Mandatory user gate and oracle

The implementer must record clean-build identity, selected mode, effective startup diagnostic, and date/session for both modes. The user must inspect the real rebuilt window:

| Mode | Required result |
|---|---|
| `CustomHidden` | Custom strip remains the product-default path; native buttons are intentionally absent; toolbar/Elements/popup/cursor/input remain usable; known maximize/restore behavior is recorded without being passed. |
| `NativeMacOsDiagnostic` | Native macOS titlebar/title and visible red/yellow/green buttons are unmistakable and non-transparent; no duplicate/custom strip or overlay exists; the egui toolbar is usable below/within the native content boundary. |
| Before animation comparison | User explicitly confirms native titlebar/buttons are present. Only then may maximize/restore animation behavior be compared. |
| Preservation | Black bar remains absent, toolbar content is not occluded, Elements remains `128.0`, toolbar right-edge/padding contracts remain intact, and popup/cursor/input behavior has no observed regression. |

**Pass** requires direct configuration and startup provenance, no option/renderer override, visible native titlebar/buttons, no custom masking, usable toolbar, preserved contracts, clean checks, and the two-mode user record. Animation is recorded only after visible chrome passes.

**Fail** if effective configuration is unproven, any option is overwritten/conflicted, native titlebar/buttons remain hidden/transparent, app paint masks/mimics native chrome, toolbar is unusable, or a preservation contract regresses. This is a diagnostic failure, not unavailable evidence.

**Blocked** only after direct configuration/ownership are proven, both clean-built modes are attempted, and a specifically evidenced external macOS compositor/platform limitation prevents meaningful observation. An unavailable display, unverified launch, or absent native buttons is not a pass and is not automatically blocked.

## Durable-document classification

- **Requirements:** `verify unchanged` — [`REQ-WRITING-LOOP-005`](../../../requirements/features/screenplay-writing-loop.md:90), [`AC-WRITING-005.5`](../../../requirements/features/screenplay-writing-loop.md:115), [`AC-WRITING-005.6`](../../../requirements/features/screenplay-writing-loop.md:131), and [`AC-WRITING-005.7`](../../../requirements/features/screenplay-writing-loop.md:137) remain the product boundary.
- **Blueprint:** `verify unchanged` — [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:83) and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:105) already describe the native/presentation boundary. Update only if an approved durable contract is actually introduced.
- **Rationale:** reversible diagnostic/configuration investigation; no product promise is created.
- **Authorized evidence:** this WO and [`execution-log.md`](../execution-log.md:1). No source or validation artifact is changed by planning.

## Fresh-session requirements and next state

Keep the initiative in planning/review. After review and required human approval, dispatch a new implementer and then a separate fresh validator. The implementer leaves this WO `in-review` with source/diff evidence, exact API audit, focused and clean checks, and the user's direct two-mode visual gate. The validator owns pass/fail and must inspect effective runtime configuration, overrides, renderer ownership, preservation, durable classification, and the native-titlebar gate.

Do not create [`validation-V3-WO-002d.md`](../validations/validation-V3-WO-002d.md:1) during planning. Next state: `plan`; no native animation result or V3-WO-002 completion claim is permitted before visible native titlebar/buttons are confirmed.

## Implementation evidence — 2026-10-27

- **Implementer session/agent ID:** `implementer-fresh-v3-wo002d-native-titlebar-direct-configuration-2026-10-27T2053Z`
- **Scope result:** The direct configuration audit found that resolved [`egui-winit-0.29.1`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-winit-0.29.1/src/lib.rs:1570) forwards decorations, transparency, and macOS titlebar/full-size-content semantics to winit, but destructures `titlebar_buttons_shown` as an intentionally ignored platform field. Therefore the diagnostic now records the requested button setting explicitly, while the visual gate remains necessary to prove the actual macOS result rather than treating the field as runtime proof.
- **Correction:** [`native_options_for_mode()`](../../../../apps/scriptcraft/src/main.rs:2563) now explicitly requests `transparent(false)` for both modes, sets `titlebar_buttons_shown(true)` for `NativeMacOsDiagnostic` and `false` for `CustomHidden`, and gives native mode the unmistakable window title `NATIVE TITLEBAR DIAGNOSTIC — ScriptCraft`; `CustomHidden` retains `ScriptCraft` and remains the default. [`startup_presentation_diagnostic()`](../../../../apps/scriptcraft/src/main.rs:2610) prints the selected mode and requested title/titlebar/buttons/decorations/transparency/full-size-content values immediately before the sole [`eframe::run_native()`](../../../../apps/scriptcraft/src/main.rs:2630) handoff.
- **Visual ownership:** The native branch still omits [`title_strip()`](../../../../apps/scriptcraft/src/main.rs:2419), its centered title, custom maximize command, and app-owned settling repaint path. The toolbar remains the only app-owned top content; no overlay, clear-color, padding, central-fill, or allocation correction was added.
- **Focused evidence:** Exact custom/native option and reversibility coverage passed; the new startup diagnostic test passed. The first focused run exposed the expected title assertion after the deliberate native diagnostic title change; the test was corrected and rerun successfully.
- **Clean checks:** `cargo clean && cargo fmt --all -- --check && cargo test -p scriptcraft && cargo check --workspace && cargo test --workspace && cargo build --workspace && cargo clippy --workspace --all-targets -- -D warnings && git diff --check` completed with exit 0. The app suite reported 89 passing tests; workspace/core/integration/doc tests also passed.
- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:2563), this work order, and [`execution-log.md`](../execution-log.md:400). No validation artifact, requirement, blueprint, core crate, closed work order, or V3-WO-001b correction was changed.
- **Required user gate / next state:** `in-review`. Set [`TITLEBAR_PRESENTATION_MODE`](../../../../apps/scriptcraft/src/main.rs:49) to `TitlebarPresentationMode::NativeMacOsDiagnostic`, clean-rebuild/relaunch, and confirm the diagnostic title plus visible non-transparent native titlebar and red/yellow/green buttons. Only after that confirmation may maximize/restore animation be compared. This implementation evidence does not claim visual success or native animation behavior; a separate fresh validator remains required.

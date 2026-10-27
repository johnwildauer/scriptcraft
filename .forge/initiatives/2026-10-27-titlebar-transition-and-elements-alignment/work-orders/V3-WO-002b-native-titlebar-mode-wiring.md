# Work Order: V3-WO-002b — Native Titlebar Mode Wiring Verification

## Metadata

- **Status:** `in-review`
- **Assignee:** Fresh desktop native-presentation wiring implementer
- **Phase/order:** Letter-suffixed fix for [`V3-WO-002`](V3-WO-002-stable-titlebar-transition-metrics.md:1), following the failed [`V3-WO-002a`](V3-WO-002a-native-titlebar-scaling-diagnostic.md:1) visual diagnostic
- **Initiative:** [`titlebar-transition-and-elements-alignment-v3.md`](../titlebar-transition-and-elements-alignment-v3.md:1)
- **Generated filename:** `V3-WO-002b-native-titlebar-mode-wiring.md`
- **Owner/orchestrator:** Forge orchestrator
- **Original work order:** [`V3-WO-002-stable-titlebar-transition-metrics.md`](V3-WO-002-stable-titlebar-transition-metrics.md:1)
- **Predecessor fix:** [`V3-WO-002a-native-titlebar-scaling-diagnostic.md`](V3-WO-002a-native-titlebar-scaling-diagnostic.md:1)
- **Validation:** create `validation-V3-WO-002b.md` only in a separate fresh validation session; this planning action creates no validation artifact
- **Implementer session/agent ID:** `implementer-fresh-v3-wo002b-native-titlebar-wiring-2026-10-27T2043Z`
- **Session exception:** none

## Summary / one outcome

Make the native-titlebar A/B diagnostic an auditable end-to-end mode rather than relying on a correctly shaped helper in isolation. Prove that the selected [`TITLEBAR_PRESENTATION_MODE`](../../../../apps/scriptcraft/src/main.rs:48) reaches [`native_options_for_mode()`](../../../../apps/scriptcraft/src/main.rs:2528), the resulting [`eframe::NativeOptions`](../../../../apps/scriptcraft/src/main.rs:2528) reaches [`eframe::run_native()`](../../../../apps/scriptcraft/src/main.rs:2546), and native mode does not paint or own the custom transparent title strip. The native macOS titlebar must be visibly present while the app's separate egui toolbar remains usable below the native chrome.

This is a wiring and verification fix, not permission to redesign the final product titlebar or claim native animation control.

## Confirmed failure being fixed

- The user attempted the [`V3-WO-002a`](V3-WO-002a-native-titlebar-scaling-diagnostic.md:1) native-titlebar A/B mode after its clean-build and deterministic checks.
- **User visual failure:** the expected native macOS titlebar was still transparent/hidden. The result is recorded as a genuine failed visual observation; it is not attributed to a misconfigured toggle and is not downgraded to unavailable display evidence.
- The observed result means the current diagnostic has not yet proved that the selected mode controls the actual startup viewport or that native title/titlebar visibility is surviving the platform path.
- Current source appears to have the intended tuple and conditional title-strip/policy branches: [`TITLEBAR_PRESENTATION_MODE`](../../../../apps/scriptcraft/src/main.rs:48), [`native_options()`](../../../../apps/scriptcraft/src/main.rs:2542), [`main()`](../../../../apps/scriptcraft/src/main.rs:2546), [`title_strip()`](../../../../apps/scriptcraft/src/main.rs:2387), and [`ScriptCraftApp::update()`](../../../../apps/scriptcraft/src/main.rs:2431). The fix must verify their runtime wiring instead of treating these local facts as proof.
- The native macOS titlebar is platform window chrome. It is not the app's custom egui toolbar. Native mode must restore the former while retaining the latter as usable app content; the toolbar must not be used as evidence that native chrome is visible.

## Required source trace and implementation boundary

The implementer must inspect the current diff and installed eframe 0.29 APIs before editing. Keep the change limited to [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1), its focused deterministic tests, this work order, and the initiative execution evidence. Do not modify validation artifacts, requirements, blueprints, closed work orders, core crates, or unrelated application behavior.

1. **Selected-mode provenance:** prove that the one-line [`TITLEBAR_PRESENTATION_MODE`](../../../../apps/scriptcraft/src/main.rs:48) selection is the single source of truth for startup. The selected enum value must be passed to the options builder; no stale hard-coded custom options, alternate startup builder, shadowed constant, or test-only path may override it.
2. **Startup handoff:** trace the exact value returned by [`native_options_for_mode()`](../../../../apps/scriptcraft/src/main.rs:2528) through [`native_options()`](../../../../apps/scriptcraft/src/main.rs:2542) and the `options` argument supplied to [`eframe::run_native()`](../../../../apps/scriptcraft/src/main.rs:2546). Add a testable seam or equivalent source-level proof for this handoff; do not rely only on inspecting the helper's tuple.
3. **Native tuple:** native diagnostic mode must request the installed eframe 0.29 APIs `.with_title_shown(true)`, `.with_titlebar_shown(true)`, and `.with_fullsize_content_view(false)`. It must retain ordinary native content presentation/scaling and must not call global egui zoom or pixels-per-point APIs. Custom mode must remain `(false, false, true)` and remain the default.
4. **App renderer boundary:** in native mode, [`ScriptCraftApp::update()`](../../../../apps/scriptcraft/src/main.rs:2431) must not paint [`title_strip()`](../../../../apps/scriptcraft/src/main.rs:2387), its centered `ScriptCraft` title text, or a transparent/full-size app-owned strip over the platform titlebar. Native mode must not expose [`title_strip_viewport_command()`](../../../../apps/scriptcraft/src/main.rs:2379) as a competing maximize/double-click owner. Custom mode retains its existing title strip, title text, and known-state command behavior unchanged.
5. **Policy boundary:** native mode must bypass the complete [`PresentationTransitionPolicy`](../../../../apps/scriptcraft/src/main.rs:1949) request/capture/hold/release and settling-repaint path, not merely suppress its diagnostics. Native maximize/restore must use platform/native window controls or double-click behavior. Custom mode retains the existing policy path.
6. **Toolbar distinction:** the top egui toolbar remains app-owned content. In native mode it must be laid out below the native titlebar/content boundary, or within the content boundary only when that is the exact installed-API result. It must remain interactive and must not paint duplicate title text, claim native titlebar ownership, or provide a competing maximize hit region.

## In scope

- Correcting mode propagation, startup option construction, or native/custom render branching discovered by the trace.
- Adding explicit mode-to-options/policy/surface helpers where needed to make the wiring deterministic and testable.
- Adding a startup/options handoff assertion or diagnostic evidence that distinguishes selected mode from the returned viewport tuple.
- Adding deterministic tests for native/custom option tuples, selected-mode reversibility, native title-strip suppression, custom title-strip preservation, native policy bypass, toolbar availability, and no global egui scale mutation.
- Rebuilding cleanly and recording the exact command/check outcomes.
- A direct user comparison of custom and native modes through maximize and restore, with a separate observation of native titlebar visibility and toolbar usability.

## Out of scope

- Any new transition algorithm, native animation-progress claim, compositor control, unsafe platform hook, desktop/work-area polling, `has_requested_resize`, global egui zoom, or pixels-per-point manipulation.
- Changing toolbar allocation, icon/text geometry, `TOOLBAR_ELEMENTS_WIDTH == 128.0`, black-bar ownership, clear color, restored bottom padding, page semantic layout, Elements behavior, popup routing, cursor ownership, input/IME/accessibility behavior, persistence, exports, or the custom-mode title-strip contract.
- Making native mode the final default, persisting the diagnostic as product state, or treating a visual success as approval for a final presentation redesign.
- Painting an app-owned black/transparent overlay to imitate or cover the native titlebar.
- Modifying any validation artifact during planning or implementation.

## Deterministic tests and required checks

Add focused tests that prove, without a display:

- custom mode is the unchanged default and native diagnostic mode is explicitly selectable and reversible;
- each mode produces the exact installed eframe viewport tuple, including title visibility, titlebar visibility, and full-size-content value;
- the selected mode is the mode used by the startup options handoff, rather than merely a mode accepted by an unused helper;
- custom mode renders/owns the app title strip and known-state maximize command, while native mode renders no app title strip, sends no custom maximize command, and leaves the toolbar path enabled;
- native mode cannot enter or repaint through the app-owned transition-policy request/capture/hold/release machinery;
- native mode leaves global egui pixels-per-point/zoom, toolbar allocation/hit rectangles, and content contracts unchanged;
- unknown maximize state remains safe and does not create a native-mode app-owned command path.

After a clean rebuild, run and record:

```text
cargo clean
cargo fmt --all -- --check
cargo test -p scriptcraft <focused mode/options/policy tests>
cargo check --workspace
cargo test --workspace
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

The exact focused-test selector and result must be recorded. Automated tests establish wiring and preservation only; they cannot establish native macOS chrome visibility or animation behavior.

## Mandatory direct visual user gate

The implementer must provide two cleanly rebuilt/relaunched selections. The user records the result, including build command/identity, selected mode, date/session, and both normal and maximize/restore observations. Do not claim native animation success before this user verification.

| Mode/action | Required direct observation |
|---|---|
| Custom hidden-titlebar mode | Confirm the app's custom title strip remains visible/usable, the platform titlebar remains intentionally hidden, the toolbar works, and record the existing maximize/restore swell/shrink/snap behavior without treating it as a pass. |
| Native-titlebar diagnostic mode | Confirm visible native macOS titlebar/window chrome, native title visibility, native maximize/restore affordance or double-click behavior, absence of the custom transparent title strip over native chrome, and a usable egui toolbar below/within the content boundary. |
| Native versus custom animation comparison | Watch the entire maximize and restore animation in both modes. Record whether toolbar icons/text and page presentation swell, shrink, or snap. Native stability is an observation only after the user reports it. |
| Preservation in both modes | Confirm the V3-WO-001b black-bar fix remains present; toolbar content is not occluded; Elements remains 128 points; toolbar right-edge/padding, custom title behavior, cursor, popup, input/IME/accessibility, and semantic contracts show no observed regression. |

**Pass:** the selected mode demonstrably reaches startup, native options request visible native title/titlebar chrome with full-size content disabled or otherwise native-compatible, native mode does not paint/own the custom title strip, the toolbar remains usable, deterministic tests and clean checks pass, and the user records both mode comparisons. Native animation may be reported only as the user's observed result; it is not presumed by the wiring pass.

**Blocked:** only after both modes are attempted from clean rebuilds and a proved external platform/compositor limitation prevents meaningful observation, with exact evidence. “No display available,” “native evidence unavailable,” or an unverified launch is not a pass.

**Fail:** the selected value does not reach [`eframe::run_native()`](../../../../apps/scriptcraft/src/main.rs:2546), the native tuple is not applied, native titlebar remains transparent/hidden, the custom strip is still painted or owns maximize in native mode, the toolbar is occluded/unusable, the policy remains active, or any preservation contract regresses.

## Durable-document classification

- **Requirements impact:** `verify unchanged` — [`REQ-WRITING-LOOP-005`](../../../requirements/features/screenplay-writing-loop.md:90), [`AC-WRITING-005.5`](../../../requirements/features/screenplay-writing-loop.md:115), [`AC-WRITING-005.6`](../../../requirements/features/screenplay-writing-loop.md:131), and [`AC-WRITING-005.7`](../../../requirements/features/screenplay-writing-loop.md:137) remain the acceptance boundary. The failed diagnostic does not approve a new final native-titlebar contract.
- **Blueprint impact:** `verify unchanged` by default — [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:83) and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:105) already bound toolbar geometry and native presentation hints. Update only the smallest evidence-backed wording if an approved final product decision creates a durable native presentation contract.
- **Rationale:** this is a reversible diagnostic wiring correction. Its implementation and user result must not be converted into a durable product promise or native animation guarantee.
- **Authorized documentation:** this fix WO and the initiative execution log; no validation artifact is modified in this planning action.

## Fresh-session and approval requirements

- Dispatch a new implementer session with the failed visual result, [`V3-WO-002a`](V3-WO-002a-native-titlebar-scaling-diagnostic.md:1), current [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1), current diff, v3 plan/review, and V3-WO-001b preservation evidence as artifacts.
- Keep the initiative in Forge planning/review until this fix WO is reviewed and required human design approval is recorded. This plan grants no implementation approval.
- The implementer must leave this WO `in-review` only after implementation evidence, deterministic tests, clean checks, and the user's direct two-mode visual record are recorded. The implementer must not mark it `complete`.
- Dispatch a separate fresh validator. The validator owns pass/fail and must inspect exact startup propagation, option tuple, renderer/title-strip boundary, policy bypass, preservation contracts, user gate, and durable classification.
- Do not create `validation-V3-WO-002b.md` during this planning action.

## Next state

`in-review` — implementation and automated evidence are recorded; a fresh validator remains required, and the user must perform the direct rebuilt custom/native A/B visual gate before any validation dispatch or completion.

## Implementation evidence

- **Branch/status before changes:** Confirmed `dev/2026-10-27-titlebar-transition-and-elements-alignment`; the existing cumulative V3 source, Forge artifacts, and V3-WO-001b black-bar evidence were preserved. No reset, checkout, commit, validation artifact, or destructive Git action was used.
- **Exact wiring diagnosis:** The prior diagnostic had the correct `(true, true, false)` tuple and conditional render branches, but only proved an isolated helper. The startup path now has one explicit `selected_titlebar_presentation_mode()` source, a `startup_presentation()` handoff that calls `native_options()` and returns the selected mode with the exact options object, and `main()` passes that object directly to `eframe::run_native()`. Native options explicitly request decorations, title shown, titlebar shown, and non-full-size content; custom remains the default `(false, false, true)` path.
- **Renderer/policy boundary:** `TitlebarSurface` and `titlebar_viewport_command_for_mode()` make native mode return no custom maximize command. The update path only paints the egui title strip and only requests `PresentationTransitionPolicy` repaint/requests in custom mode. Native mode continues through the existing toolbar/content path with no duplicate title strip or competing maximize owner.
- **Preservation:** No black-bar correction, clear-color owner, restored `TOOLBAR_BOTTOM_PADDING = 2.0`, toolbar allocation, `TOOLBAR_ELEMENTS_WIDTH = 128.0`, Elements, popup, cursor, input/IME/accessibility, semantic layout, or custom policy algorithm was changed.
- **Deterministic tests:** Exact custom/native viewport tuple and explicit decorations; selected startup-mode-to-options handoff; reversible surface selection; custom command versus native command suppression; custom policy repaint versus native bypass; and existing unknown-state safety. Focused selectors `native_options_are_exact_and_reversible_for_both_modes`, `titlebar`, and `title_strip_viewport_toggle_is_safe_for_known_and_unknown_state` passed.
- **Checks:** An initial multi-test-name Cargo invocation was rejected by Cargo argument parsing (no test execution); corrected focused runs passed. After `cargo clean`, `cargo fmt --all -- --check`, `cargo test -p scriptcraft` (88 passed), `cargo check --workspace`, `cargo test --workspace`, `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` all exited 0.
- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:47), this work order, and [`execution-log.md`](../execution-log.md:1). No validation artifact, requirement, blueprint, core crate, or closed work order was changed.
- **User gate remains outstanding:** No native visual success is claimed. The user must clean-rebuild and compare both modes through maximize/restore, native titlebar visibility, toolbar usability, black-bar absence, and snap behavior before validation is dispatched.

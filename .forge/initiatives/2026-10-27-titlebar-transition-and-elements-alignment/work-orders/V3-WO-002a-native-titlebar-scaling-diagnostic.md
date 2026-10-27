# Work Order: V3-WO-002a — Native Title-Bar and Scaling Diagnostic

## Metadata

- **Status:** `in-review`
- **Assignee:** Fresh desktop native-presentation diagnostic implementer
- **Phase/order:** Letter-suffixed fix for V3-WO-002; follows the failed V3-WO-002 visual gate
- **Initiative:** [`titlebar-transition-and-elements-alignment-v3.md`](../titlebar-transition-and-elements-alignment-v3.md:1)
- **Generated filename:** `V3-WO-002a-native-titlebar-scaling-diagnostic.md`
- **Owner/orchestrator:** Forge orchestrator
- **Original work order:** [`V3-WO-002-stable-titlebar-transition-metrics.md`](V3-WO-002-stable-titlebar-transition-metrics.md:1)
- **Failed-parent evidence:** [`../execution-log.md`](../execution-log.md:301) records the V3-WO-002 implementation and user visual failure; the new failure is recorded in the next execution-log entry.
- **Validation:** create `validation-V3-WO-002a.md` only in a separate fresh validation session; this planning action does not create a validation artifact.
- **Implementer session/agent ID:** `implementer-fresh-v3-wo002a-native-titlebar-2026-10-27T2034Z`
- **Session exception:** none

## Summary / one outcome

Provide a reversible A/B diagnostic that compares the existing custom hidden/transparent title-bar presentation against an installed-API native macOS title-bar/content-scaling presentation. The diagnostic must make the native mode explicit, bypass the app-owned [`PresentationTransitionPolicy`](../../../../apps/scriptcraft/src/main.rs:1949) so that policy cannot confound the native comparison, keep the custom toolbar available below or within the native title-bar boundary as safely as the eframe API permits, and return a user-observed finding. This is an evidence-gathering fix WO, not authorization for a final UI redesign or a promise to control native animation internals.

## Failed-parent diagnosis and user failure

- The parent [`V3-WO-002-stable-titlebar-transition-metrics.md`](V3-WO-002-stable-titlebar-transition-metrics.md:1) implemented an app-owned combined transition policy and passed its deterministic tests and clean Cargo gates, but it failed the required real visual gate.
- **User visual failure:** In the rebuilt app, the combined app-owned transition policy did not stop toolbar/page swelling, shrinking, or snapping during real maximize and restore. The visual behavior therefore remains failed; it is not an unavailable-native-evidence warning and cannot be passed from the automated metric/state tests.
- The failure does not prove that native title-bar/scaling behavior is the final fix. It establishes the need for a bounded A/B diagnostic that removes the custom hidden/transparent title-bar path and the app-owned transition-policy variable from the comparison.
- Preserve the confirmed V3 black-bar result from [`V3-WO-001b-toolbar-black-bar-source-trace.md`](V3-WO-001b-toolbar-black-bar-source-trace.md:1). This WO must not reopen the black-bar diagnosis or use padding removal, central-fill changes, clear-color changes, or toolbar compression as a transition remedy.

## In scope

1. Add one explicit, reversible native-titlebar diagnostic configuration in [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1). The implementation may use a clearly named compile-time constant or equivalent explicit configuration selected at startup; it must be easy to switch between modes and require a rebuild/relaunch. Do not silently alter the final product default or persist the diagnostic as user/project state.
2. In **custom mode**, preserve the current hidden/transparent/full-size-content path and the app-owned title-strip/transition behavior as the comparison baseline.
3. In **native-titlebar A/B mode**, use only installed eframe 0.29 viewport options to request native macOS presentation:
   - `.with_title_shown(true)`;
   - `.with_titlebar_shown(true)`;
   - `.with_fullsize_content_view(false)`;
   - retain ordinary native content presentation/scaling; do not call global egui zoom or pixels-per-point APIs.
4. Bypass or disable [`PresentationTransitionPolicy`](../../../../apps/scriptcraft/src/main.rs:1949) in native-titlebar mode. The bypass must cover request/capture/hold/release behavior and repaint-for-settling behavior, not merely hide its diagnostics. Native macOS maximize/restore must be allowed to use the platform/native window path rather than the custom title-strip request path.
5. Keep the custom toolbar rendered in native mode below the native titlebar, or within the native content boundary if that is the only safe installed-API result. Do not add a second app-owned title strip in native mode; avoid duplicate title text or a competing maximize hit region. The existing custom hidden-titlebar mode retains its title strip.
6. Keep the two modes selectable without changing the product presentation contract implicitly. The selected mode must be visible in source/configuration and recorded in implementation evidence, and the implementation must document the one-line toggle/rebuild procedure.

## Out of scope

- Claiming native animation progress, interpolation, suppression, frame-history restoration, compositor control, or a final UI fix.
- Rewriting [`PresentationTransitionPolicy`](../../../../apps/scriptcraft/src/main.rs:1949) into a new transition algorithm as part of the diagnostic; it may be preserved and explicitly bypassed/disabled for native mode.
- Global `Context::set_zoom_factor()` / `set_pixels_per_point()`, desktop/work-area polling, `has_requested_resize`, platform-specific unsafe hooks, or unsupported native APIs.
- Changing toolbar allocation, icon/text geometry contracts, title-strip geometry in custom mode, Elements behavior, popup routing, cursor ownership, input/IME/accessibility behavior, semantic editing, persistence, exports, page layout, or black-bar ownership.
- Making native mode the final default or silently changing the final product presentation contract.
- Modifying any validation artifact, requirements file, blueprint, v1/v2 artifact, or closed work order during implementation unless a separately authorized evidence reconciliation permits it. This planning action modifies no source or validation artifact.

## Likely files and exact source boundaries

- Primary implementation and tests: [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1).
- Native/custom viewport construction: [`native_options()`](../../../../apps/scriptcraft/src/main.rs:2471), including the installed [`ViewportBuilder`](../../../../apps/scriptcraft/src/main.rs:2473) title/titlebar/full-size-content fields.
- App startup selection: [`main()`](../../../../apps/scriptcraft/src/main.rs:2484) and any small app-local mode helper introduced by the implementer.
- Custom title-strip and maximize request path: [`title_strip()`](../../../../apps/scriptcraft/src/main.rs:2339) and [`title_strip_viewport_command()`](../../../../apps/scriptcraft/src/main.rs:2331). Native mode must not retain a competing custom title-strip command surface.
- App-owned policy seam: [`PresentationTransitionPolicy`](../../../../apps/scriptcraft/src/main.rs:1949), [`title_strip()`](../../../../apps/scriptcraft/src/main.rs:2371), [`ScriptCraftApp::update()`](../../../../apps/scriptcraft/src/main.rs:2383), and the renderer consumers in [`live_document_surface()`](../../../../apps/scriptcraft/src/main.rs:1411).
- Existing renderer metric helpers: [`ToolbarPresentationMetrics`](../../../../apps/scriptcraft/src/main.rs:1892), [`PresentationMetricSet`](../../../../apps/scriptcraft/src/main.rs:1934), and [`metric_set_close()`](../../../../apps/scriptcraft/src/main.rs:2063). These are diagnostic comparison points, not permission to broaden native scaling into global egui scaling.
- Dependency/API evidence: [`apps/scriptcraft/Cargo.toml`](../../../../apps/scriptcraft/Cargo.toml:8), which pins the direct eframe dependency to `0.29`; installed eframe/egui 0.29 options must be verified rather than guessed.

## Configuration and policy-selection contract

- Define a named mode such as `TitlebarPresentationMode::CustomHidden` and `TitlebarPresentationMode::NativeMacOsDiagnostic`, with the diagnostic choice explicit at compile time or through a documented build-time configuration. A runtime environment toggle is acceptable only if the selected value is deterministic, visible in startup evidence, and does not become persisted product state.
- The default/final-product selection remains the current custom hidden-titlebar mode unless the user separately approves another default. The native mode is an interim A/B diagnostic only.
- `native_options()` must be derived from the selected mode. Tests must assert the custom tuple `(title_shown=false, titlebar_shown=false, fullsize_content_view=true)` and native diagnostic tuple `(true, true, false)`.
- Policy selection must be explicit and testable: custom mode selects the current app-owned policy; native diagnostic mode selects a no-op/bypassed policy path. Tests must prove native mode cannot enter `Requested`, `Settling`, or `Released` as a consequence of title-bar maximize/restore handling and cannot request repaint solely to settle the app-owned policy.
- Native mode must use native window maximize/restore affordances. If the native titlebar does not expose a usable app-owned double-click surface, that is an observation to record, not a reason to add a custom competing title strip.
- The selected mode and the exact option tuple must be stated in the work-order implementation evidence and in the user visual record.

## Deterministic tests and required checks

- Add focused tests for mode selection and reversibility: custom is the unchanged default; native diagnostic is explicitly selectable; each selection produces the exact installed eframe 0.29 viewport option tuple; switching the configuration and rebuilding changes only the intended presentation path.
- Add focused tests for policy selection/bypass: custom mode retains app-owned policy behavior; native mode uses the bypass/no-op path; unknown maximize state remains safe; no native-mode path calls the custom request/capture/settling/release machinery.
- Add focused tests for title-strip selection: custom mode renders the app-owned title strip and owns its known-state command path; native mode does not render a duplicate app title strip or send its custom maximize command, while the toolbar remains enabled.
- Preserve and rerun deterministic tests for the confirmed black-bar clear-color/padding contract, toolbar allocation/right-edge behavior, title/custom-mode geometry, `TOOLBAR_ELEMENTS_WIDTH == 128.0`, Elements closed/open/popup routing, cursor ownership, input/IME/accessibility boundaries, semantic layout, and page minimum-width behavior.
- Required project gates after a clean rebuild: `cargo clean`; `cargo fmt --all -- --check`; focused mode/options/policy tests; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; and `git diff --check`.
- Deterministic tests do not constitute the visual pass. They establish configuration/policy selection and preservation only.

## Mandatory user A/B visual gate

The implementer must provide two cleanly rebuilt/relaunched binaries or two explicitly recorded rebuild selections. The user, not the implementer, records the visual result. The evidence must include build identity/command, selected mode, date/session, and observations for both modes.

| Mode and action | Required observations |
|---|---|
| Custom hidden-titlebar mode — maximize then restore using the existing custom title-strip path | Record whether toolbar icons/text and page presentation swell, shrink, or snap; record final normal/maximized sizes; retain the known failure if reproduced. |
| Native-titlebar diagnostic mode — maximize then restore using native macOS titlebar controls/double-click behavior | Record whether toolbar icons/text and page sizes remain stable, whether the snap disappears, whether native scaling changes are visible, and whether any transition still swells/shrinks. |
| Native mode layout comparison | Record native title visibility, toolbar position below/within the native titlebar, duplicate/missing title-strip behavior, toolbar/title spacing, content origin, page bounds, and any change to Elements/popup/input/cursor behavior. |
| Preservation check in both modes | Confirm the black-bar fix remains present, toolbar content is not occluded, Elements remains 128 points, toolbar right-edge/padding contracts remain intact, and title/custom input/popup/cursor behavior has no observed regression. |

**Pass:** the native A/B configuration is explicit and reversible; the policy is bypassed in native mode; both clean builds pass all required checks; the user records both comparisons; and the result clearly states whether native mode removes the swell/shrink/snap without violating preservation contracts.

**Blocked:** only if the user attempts both modes on a clean rebuild and a proved external platform/compositor limitation prevents a meaningful comparison, with exact evidence recorded. “No display available” or “native evidence unavailable” without an attempted user gate is not a pass.

**Fail:** native mode still exhibits the same swell/shrink/snap, the app-owned policy remains active or confounds the comparison, the toggle is implicit/non-reversible, native titlebar options are not the exact installed API mapping, the toolbar/title layout is unsafe, or any preserved contract regresses.

## Bounded next step after comparison

- If native mode removes the swell/shrink/snap and preserves the required contracts, retain native mode as a **diagnostic finding only** and record that the custom hidden-titlebar/app-owned policy path is implicated. Do not silently make native mode the final product default.
- If native mode reproduces the defect, or both modes show the same behavior, use the controlled comparison evidence to revise the app-owned policy in a separately scoped follow-up. That follow-up must define a new bounded hypothesis; it must not promise native animation internals or treat this diagnostic as the final UI fix.
- If native mode has unacceptable title/toolbar layout or contract regressions while resolving the snap, retain the result as a diagnostic finding and return to product/design review before changing the final presentation contract.

## Durable-document classification

- **Requirements impact:** `verify unchanged` — [`REQ-WRITING-LOOP-005`](../../../requirements/features/screenplay-writing-loop.md:90), [`AC-WRITING-005.5`](../../../requirements/features/screenplay-writing-loop.md:115), [`AC-WRITING-005.6`](../../../requirements/features/screenplay-writing-loop.md:131), and [`AC-WRITING-005.7`](../../../requirements/features/screenplay-writing-loop.md:137) remain the comparison boundary. Update only if an approved final product decision changes a user-facing contract; the diagnostic finding alone is not that decision.
- **Blueprint impact:** `verify unchanged` by default — [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:83), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:105), and [`technical-context.md`](../../../blueprints/technical-context.md:31) already bound the native viewport hints, title/toolbar surface, presentation layer, and semantic layout. Apply only the smallest evidence-backed update if the final approved documents need to record the diagnostic configuration or a durable native presentation decision.
- **Affected documents/IDs:** REQ-WRITING-LOOP-005 / AC-WRITING-005.5–005.7; desktop viewport/presentation contract.
- **Rationale/evidence:** This WO is a reversible diagnostic and must not convert an A/B observation into a final product promise. The final classification must match the implementation diff and the user visual record.
- **Authorized documentation changes:** this fix WO and the execution log; conditional minimal evidence reconciliation in requirements/blueprints only if a durable contract is actually approved. No validation artifact is modified in this planning action.

## Fresh-session and approval requirements

- This fix requires a new implementer session and a separate fresh validator session. The implementer must receive the failed parent, the failed user visual result, the current source/diff, the V3 plan/review, and the confirmed V3-WO-001b preservation evidence as artifacts.
- The initiative must remain in the Forge planning/review gate until the fix WO is reviewed and any required human design approval is recorded. This planning action grants no implementation or design approval.
- The implementer must not mark this WO complete. After implementation, leave it `in-review` only with source/diff, clean rebuild, deterministic tests, and the user's two-mode visual record documented.
- The fresh validator owns pass/fail and must inspect the exact configuration/policy boundary, required gates, user A/B record, preservation contracts, and durable-document classification. A missing user A/B record is non-complete.
- Do not create `validation-V3-WO-002a.md` during this planning action.

## Next state

`plan` — fix WO created after the failed V3-WO-002 visual gate. Review this handoff and obtain the required approval before dispatching a fresh implementer; no source implementation, validation, or final design approval is authorized here.

## Implementation evidence — 2026-10-27

- **Branch/status:** Verified `dev/2026-10-27-titlebar-transition-and-elements-alignment` before changes. The existing V3-WO-001b black-bar fix, including `TOOLBAR_BOTTOM_PADDING = 2.0` and the `ScriptCraftApp::clear_color()` owner, was preserved. No reset, checkout, commit, or destructive Git action was used.
- **Explicit configuration:** [`TITLEBAR_PRESENTATION_MODE`](../../../../apps/scriptcraft/src/main.rs:48) is the one-line compile-time switch and remains `TitlebarPresentationMode::CustomHidden` by default. Set it to `TitlebarPresentationMode::NativeMacOsDiagnostic`, rebuild, and relaunch for the A/B diagnostic; restore `CustomHidden` and rebuild/relaunch to return to the custom comparison path.
- **Native tuple:** [`native_options_for_mode()`](../../../../apps/scriptcraft/src/main.rs:2504) uses installed eframe/egui 0.29 APIs exactly: custom `(title_shown=false, titlebar_shown=false, fullsize_content_view=true)`; native diagnostic `(true, true, false)`. No global egui zoom or pixels-per-point API was added.
- **Presentation/policy boundary:** Native mode omits the app-owned title strip and its maximize command surface, leaves the toolbar as the top app-owned content below the native titlebar/content boundary, uses live toolbar/page metrics, skips policy observation/capture/hold/release, and does not request settling repaints. Custom mode retains the existing title strip, viewport command semantics, and [`PresentationTransitionPolicy`](../../../../apps/scriptcraft/src/main.rs:1981) behavior.
- **Preservation:** Toolbar allocation, Elements `128.0` width, title/custom hit behavior, popup/cursor/input/IME/accessibility/semantic behavior, page minimum-width behavior, clear-color, and restored toolbar padding were not changed. No transition algorithm, black-bar correction, or durable requirement/blueprint contract was broadened.
- **Deterministic evidence:** Added exact option-tuple/reversibility coverage and explicit custom-versus-native mode/policy selection tests. Native-mode metric consumers return live values and the native mode cannot enter the app-owned policy through the titlebar path.
- **Checks:** `cargo clean`; `cargo fmt --all`; `cargo fmt --all -- --check`; focused mode/options/policy tests (2 passed); `cargo test -p scriptcraft` (88 passed); `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; and `git diff --check` all passed with exit 0. A first post-edit clean chain stopped at rustfmt drift only; formatter correction was applied and the complete clean chain then passed.
- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:48), this work order, and [`execution-log.md`](../execution-log.md:325). No validation artifact was created; requirements and blueprints remain `verify unchanged`.
- **User gate / next state:** No visual result is claimed here. The work order is `in-review`; stop for the user's two-mode clean-rebuild visual comparison before dispatching validation or final V3-WO-002 validation.

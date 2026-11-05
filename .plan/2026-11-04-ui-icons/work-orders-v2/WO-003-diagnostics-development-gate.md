# V2-WO-003 — Diagnostics development and production gate

- **Status:** Implementation submitted on 2026-11-05 after V2-WO-002 handoff; user review and validation pending
- **Initiative:** [`ui-icons-v2.md`](../ui-icons-v2.md)
- **Depends on:** V2-WO-002 implementation handoff
- **Validation owner:** User; no agent-run builds, checks, tests, lint, formatting checks, manual verification, reviews, or validators

## Outcome

Add one named manual compile-time boolean that disables the status Diagnostics affordance and its detailed presentation projection in production while preserving an explicit development-on mode and all operational counters.

## Scope

1. Add `ENABLE_DIAGNOSTICS: bool = false` in the existing [`presentation`](../../../apps/scriptcraft/src/status_bar.rs:19) module, with comments identifying production-off, explicit development-on, and rebuild/relaunch requirements. Expose one narrow parent-visible accessor rather than publishing the whole module.
2. In [`show_application_panels()`](../../../apps/scriptcraft/src/main.rs:2484), skip the detailed performance string, pending-derived-work presentation query, and [`DiagnosticProjection`](../../../apps/scriptcraft/src/main.rs:2523) construction when disabled. Use an optional projection or equivalent explicit disabled projection boundary without fabricating empty diagnostics.
3. In [`show_impl_with_appearance()`](../../../apps/scriptcraft/src/status_bar.rs:1248), honor the same constant and render no collapsing header, body, placeholder allocation, focus target, or accessibility node when disabled.
4. When enabled, preserve the existing collapsed default, egui-memory disclosure state, safe summary, clipping, keyboard/pointer behavior, accessibility, and footer sizing.
5. Preserve [`PerformanceDiagnostics`](../../../apps/scriptcraft/src/main.rs:295), [`NativeInputDiagnostics`](../../../apps/scriptcraft/src/main.rs:88), operational counters, repaint/layout scheduling, and all non-Diagnostics status/error behavior. The gate covers presentation and projection, not application instrumentation removal.
6. Do not use `cfg!(debug_assertions)` or add a Cargo feature. This is an explicit source-controlled product choice, independent of build profile.
7. Preserve the right-edge trace until V2-WO-004, the stable centered percentage slot, all manual values, and all existing zoom/knob behavior.
8. Update the Diagnostics-conditional portions of [`AC-WRITING-006.1`](../../../.forge/requirements/features/screenplay-writing-loop.md), [`AC-WRITING-006.3`](../../../.forge/requirements/features/screenplay-writing-loop.md), [`AC-WRITING-006.4`](../../../.forge/requirements/features/screenplay-writing-loop.md), and [`AC-WRITING-006.9`](../../../.forge/requirements/features/screenplay-writing-loop.md), plus the scoped observability/status sections in [`technical-context.md`](../../../.forge/blueprints/technical-context.md), the initiative, and this handoff.

## Non-goals

- No runtime preference, hidden command, document persistence, manifest feature, dependency change, or global observability removal.
- No margin or percentage-layout correction beyond preserving V2-WO-002.
- No agent-run validation.

## Structural acceptance for user review

1. Production-off has no visible/focusable/accessibility Diagnostics row and no detailed projection construction or reserved row height.
2. Development-on restores the previous complete disclosure behavior.
3. Operational counters and primary status/error/zoom behavior remain intact in both modes.
4. Switching occurs only by editing the constant and rebuilding; document state is unaffected.

## Implementation handoff — 2026-11-05

Implemented on the existing shared `dev/2026-11-04-ui-icons` branch. This handoff records source and scoped documentation changes only. No build, check, test, lint, formatting command, application run, manual verification, review, or other validation was run or claimed. The user owns all review and validation under the project policy.

### Gate and code locations

- [`presentation::ENABLE_DIAGNOSTICS`](../../../apps/scriptcraft/src/status_bar.rs:47) is the sole boolean, set to `false`, with comments specifying production-off, explicit local development-on, and rebuild/relaunch requirements. [`diagnostics_enabled()`](../../../apps/scriptcraft/src/status_bar.rs:258) is the narrow parent-visible accessor; the presentation module remains private.
- [`show_application_panels()`](../../../apps/scriptcraft/src/main.rs:2484) checks that accessor before formatting the detailed status summary or querying pending derived work for presentation. It builds an optional [`DiagnosticProjection`](../../../apps/scriptcraft/src/main.rs:2525) only when enabled; disabled mode creates no substitute or empty projection. Operational instrumentation and application scheduling remain active.
- [`show_impl_with_appearance()`](../../../apps/scriptcraft/src/status_bar.rs:1463) uses the same module constant to guard the complete Diagnostics disclosure. When disabled it creates no header/body, reserved row height, focus target, or accessibility node. When enabled it requires a supplied projection and retains the existing collapsed default, egui disclosure state, clipped safe summary, keyboard/pointer/accessibility behavior, and footer sizing.
- `PerformanceDiagnostics`, `NativeInputDiagnostics`, repaint/layout scheduling, existing error/status behavior, V2-WO-001 geometry tracing, V2-WO-002 stable centered percentage geometry, WO-007 knob behavior, and all manually adjusted numeric/color values are preserved. V2-WO-004's width/margin correction and trace removal were not performed.

### Scoped records and pending validation

Updated only the Diagnostics-conditional language in AC-WRITING-006.1/.3/.4/.9, the status and observability sections of [`technical-context.md`](../../../.forge/blueprints/technical-context.md), the Diagnostics sections and implementation handoff in [`ui-icons-v2.md`](../ui-icons-v2.md), and this handoff. The baseline and post-slot trace files remain untouched. All user-owned review and validation remains pending.

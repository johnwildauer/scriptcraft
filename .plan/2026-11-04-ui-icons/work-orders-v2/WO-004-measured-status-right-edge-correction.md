# V2-WO-004 — Measured status right-edge correction

- **Status:** Implementation submitted on 2026-11-05 after the user-trace evidence gate; user review and validation pending
- **Initiative:** [`ui-icons-v2.md`](../ui-icons-v2.md)
- **Depends on:** V2-WO-001 and V2-WO-002 implementation handoffs plus user-supplied traces from both stages
- **Validation owner:** User; no agent-run builds, checks, tests, lint, formatting checks, manual verification, reviews, or validators

## Outcome

Correct the specific allocation, alignment, or clipping layer shown by the geometry traces to create the width-independent right-edge gap. Do not ship a guessed 90px subtraction or hide the defect by reducing unrelated constants.

## Entry evidence gate

Before changing geometry, the implementation agent must receive the user's V2-WO-001 baseline trace and a second trace after V2-WO-002, including effective scale and edge references. If either is missing, stop and request that specific evidence. Reading user-supplied traces for implementation diagnosis is permitted; the agent must not run the application or treat diagnosis as validation.

The screenshot confirms the visible symptom. The traces identify the owner.

## Scope

1. Compare the first nonzero deltas through the chain: native/egui content, panel frame/content, available/row, right group, zoom subgroup/nested horizontal UI, actual percentage rectangle, galley advance, visible mesh/ink, and final clip.
2. Distinguish three separate measurements:
   - percentage **slot/right-group edge** to egui content edge;
   - centered galley advance edge to its slot edge, which intentionally varies after V2-WO-002;
   - native outer window edge to egui content edge and physical screenshot scaling.
3. If an actual child or response rectangle differs from its planned rectangle, make the planned explicit rectangle authoritative at that owning layer or correct the specific layout/clip misuse. Preserve the stable slot, centered text, control priority, and nonoverlap.
4. If clipping alone shortens visible text while allocation is correct, repair only the owning clip. Do not change allocation constants to compensate for clipped ink.
5. If traces show that app-owned slot geometry is already correct and the residual is native outer/content conversion, screenshot scaling, or intentional centered-label whitespace, make no geometry change. Record the complete accounting instead.
6. Preserve every current manual presentation value. No arbitrary residual constant, negative inset, fixed screenshot-scale assumption, or global panel/toolbar change is permitted.
7. Remove the temporary V2-WO-001 trace from the final production path after recording its evidence and correction rationale in the handoff. Do not retain unsolicited per-frame logging.
8. Update only the measured-owner portions of [`AC-WRITING-006.4`](../../../.forge/requirements/features/screenplay-writing-loop.md), [`AC-WRITING-006.7`](../../../.forge/requirements/features/screenplay-writing-loop.md), and [`technical-context.md`](../../../.forge/blueprints/technical-context.md), plus the initiative and this handoff. If no app defect is proven, record that outcome without fabricating a code fix.

## Non-goals

- No change based solely on the estimated 90px screenshot distance.
- No fixed-width or Diagnostics work beyond preserving predecessor orders.
- No zoom, popup, accessibility, persistence, pagination, or export redesign.
- No agent-run validation.

## Structural acceptance for user review

1. The handoff names the first unexpected trace delta and maps each code change directly to it, or records why no app-owned defect exists.
2. Slot/right-group geometry remains anchored to the actual egui content edge at multiple widths; centered-label whitespace is evaluated separately.
3. No manual numeric/color value changes and no temporary production trace remains.
4. User review covers multiple widths/scales, all percentage digit counts, narrow clipping, popup/knob behavior, and native outer/content edge accounting.

## Implementation handoff — 2026-11-05

Implemented on the existing shared `dev/2026-11-04-ui-icons` branch. The user-supplied [`baseline trace`](W0-002-grep-report.txt) and [`post-slot trace`](WO-002-post-slot-trace.txt) were preserved unchanged. This is a source/documentation handoff only: no build, application run, test, lint, formatting check, manual verification, review, or validation was performed or claimed. All validation remains user-owned under the authoritative project policy.

### Measured owner and correction

- Baseline evidence at egui content width 1512 recorded planned slider width 165 and actual width 100; the actual percentage rectangle ended 65 points before its planned right edge, and the galley advance ended 85 points before the content/window right at effective ppp 2.0.
- After V2-WO-002, the supplied post-slot trace at 800×600 and 100% zoom recorded planned and actual percentage rectangles both at `[746.78..788.00]`. The percentage right-edge owner was already corrected; no additional panel or percentage inset defect was present in that trace.
- The remaining first mismatch was specifically the stock slider: planned end 742.78 versus actual end 677.78, a 65-point width shortfall. With the intended 12-point panel inset and 8-point centered-galley reservation, that explains the baseline's 85-point gap. There is no `subtract 90`, inset reduction, or other margin compensation.
- [`zoom_slider()`](../../../apps/scriptcraft/src/status_bar.rs) now sets the public egui 0.34 `Spacing::slider_width` value on the scoped slider UI to the planner-provided width before adding the stock `Slider`. In egui 0.34, `Slider` allocates its response using this spacing field (whose default is 100 points); the existing `add_sized` constrained the child but did not override that internal width. The planner's existing 40–165-point value is used directly, with no new numeric constant. Stock response, input, accessibility, range, and zoom semantics remain in place.
- The centered percentage slot, exact planned percentage rectangle, popup anchor/routing, full interaction/accessibility response, WO-007 deferred knob geometry/paint priority and clipping, native pinch, zoom persistence, Diagnostics gate, and every user-adjusted numeric/color value were preserved. The only source behavior change is matching the stock slider response width to the planner allocation.

### Temporary trace removal and limitations

After recording the evidence and rationale above, removed the V2-WO-001 production switch, trace structures/helpers, status capture arguments/assignments, and main-panel capture/emission call. No per-frame or main-panel trace logging remains in the production path. The baseline and post-slot trace files remain intact. The available post-slot trace covers only 800×600, 100% zoom, and effective ppp 2.0; it is not a post-fix observation. Behavior at other widths/scales, compressed widths, popup/knob details, and native outer/content measurement remain for user-owned review and validation.

### Scoped records updated

Updated only the measured-owner language in AC-WRITING-006.4/.7, the corresponding status sizing/evidence paragraph in technical context, the V2 initiative status/findings and implementation handoff, and this handoff. No other requirement, manifest policy, predecessor handoff, evidence file, or unrelated work was changed.

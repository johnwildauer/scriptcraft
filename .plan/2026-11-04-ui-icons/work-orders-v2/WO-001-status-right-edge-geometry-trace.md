# V2-WO-001 — Status right-edge geometry trace

- **Status:** Implemented on 2026-11-05; user evidence and validation pending
- **Initiative:** [`ui-icons-v2.md`](../ui-icons-v2.md)
- **Depends on:** WO-008 implementation handoff
- **Validation owner:** User; no agent-run builds, checks, tests, lint, formatting checks, manual verification, reviews, or validators

## Outcome

Add temporary, geometry-only development tracing that identifies the first unexpected right-edge delta between the native content edge, bottom panel, planned status row, actual child UIs, percentage allocation, and painted galley. This order changes no production layout and applies no compensating margin.

The user-supplied screenshot confirms a substantial visible gap, but it does not identify the owning rectangle or provide physical/logical scale. One instrumented user run is the remaining evidence needed before a measured correction.

## Scope

1. Add a clearly named temporary trace switch in [`presentation`](../../../apps/scriptcraft/src/status_bar.rs:19), initially enabled for the evidence build and documented for removal in V2-WO-004. Do not conflate it with the later Diagnostics production gate.
2. Capture one coherent same-frame ledger, in logical points, for:
   - native viewport inner/outer rectangles where exposed, egui screen/content rectangle, native and effective pixels per point, and UI zoom;
   - bottom panel outer response rectangle, panel content maximum rectangle, and inherited clip;
   - available rectangle, planned row, planned right group, slider, percentage, and gaps;
   - actual main-row child, right-group child, zoom subgroup, nested horizontal child, slider response, percentage allocation/interaction rectangle, and painter clip;
   - actual current label, galley origin/size/advance right, and mesh/visible bounds where publicly available.
3. Emit derived right-edge deltas so the first unexpected owner is obvious: panel outer-to-content, content-to-row, row-to-right-group, right-group-to-zoom-child, zoom-child-to-percentage, percentage-to-galley, and galley-to-visible-ink.
4. Avoid per-frame log flooding. Emit only when the geometry/signature changes or through another bounded development-only mechanism. Do not log document text, paths, error details, timer content, or other project data.
5. Keep the trace inside the existing status path in [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs) and the panel/native boundary in [`show_application_panels()`](../../../apps/scriptcraft/src/main.rs:2484). Preserve all painting, input, accessibility, popup, zoom, and numeric/color behavior.
6. Provide exact user instructions for running the evidence build, resizing to several widths, and supplying the trace together with screenshot/output scaling. Request captures at the user's normal window size plus 1200, 480, and 320 logical-point widths where practical.
7. Record the instrumentation locations and evidence handoff in [`ui-icons-v2.md`](../ui-icons-v2.md) and this work order. Do not claim the roughly 90px owner has been found until the user supplies measurements.

## Non-goals

- No layout correction, inset change, magic offset, fixed percentage slot, or Diagnostics gate.
- No automated desktop-test target and no agent execution of the application or validation.
- No persisted document/application preference and no broad telemetry facility.

## User evidence handoff

The implementation agent stops after adding the trace and this handoff. Instrumentation is in [`status_bar.rs`](../../../apps/scriptcraft/src/status_bar.rs): the `presentation::ENABLE_STATUS_GEOMETRY_TRACE` switch is initially `true`, status geometry is gathered in `show_impl_with_appearance()`, and percentage galley/clip metrics are captured in `zoom_presets_button()`. Native/window/panel boundary data and final ledger emission are in [`show_application_panels()`](../../../apps/scriptcraft/src/main.rs). Lines are written to stderr with prefix `[V2-WO-001 status geometry]`; changed signatures are rate-limited to at most one line per 250 ms, with unchanged frames silent. No geometry, paint, input, accessibility, popup, zoom, color, or presentation values were changed.

### Exact user evidence instructions

1. On the existing `dev/2026-11-04-ui-icons` branch, use the user's normal local build/run workflow from the repository root. To capture the designated stderr stream, run `cargo run -p scriptcraft 2> /tmp/scriptcraft-v2-wo-001.stderr`; keep the newly launched window open and do not rely on a stale process/build.
2. Capture the normal window size and, where practical, approximately 1200, 480, and 320 logical-point egui content widths. Read actual dimensions from `egui_content`/`available` in the ledger. Let the UI render after each resize. Capture one additional zoom value at one width.
3. Capture a corresponding native screenshot at each width. Report the native/monitor scale, screenshot/output rescaling, and whether the measured right edge is the native outer edge or egui content edge.
4. After closing the app, extract only trace lines with `grep '^\[V2-WO-001 status geometry\]' /tmp/scriptcraft-v2-wo-001.stderr`. Supply those lines, screenshots, and scale/reference notes; do not provide unrelated stderr output.
5. The trace logs UI zoom percentage and geometry/scale only. It does not log document text, paths, error details, timer content, or other project content.

### Instrumentation limitations and current state

Native viewport bounds may be unavailable and, when present, may use a coordinate origin different from local egui content coordinates. Effective/native pixels-per-point and derived UI zoom are reported separately; unavailable native scale is not represented as a measured value. Mesh and visible bounds are geometric (the latter is mesh bounds intersected with the captured clip), not post-rasterization ink pixels. Screenshot scaling and native decoration thickness still require user-reported evidence. The trace has not been run; no right-edge owner is established. The user owns compilation, execution, trace/screenshot capture, review, and all validation. V2-WO-004 must not proceed to a correction without the user's trace. This work order did not implement V2-WO-002, V2-WO-003, or any geometry correction/margin compensation.

## Structural acceptance for user review

1. The trace reports planned and actual right edges in one coordinate system and includes effective scale.
2. Repeating at different widths shows whether each delta is fixed or variable.
3. The application otherwise behaves identically and no user-adjusted presentation value changes.
4. No sensitive document data appears in the trace.

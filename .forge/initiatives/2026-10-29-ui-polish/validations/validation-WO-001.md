# Validation: WO-001 — Scale and Page Geometry

## Validation metadata

- **Review type:** `validation`
- **Status:** `complete`
- **Validator:** Fresh independent Forge validator
- **Validator session/agent ID:** Not available in this interface; not inferred
- **Session exception:** `none`
- **Date:** `2026-10-29`
- **Initiative:** [UI Polish plan](../ui-polish-v1.md)
- **Validated work order:** [WO-001 — Scale and Page Geometry](../work-orders/WO-001-scale-page-geometry.md)
- **Compared evidence:** Current working-tree diff against `HEAD` for the reported product sources, the recorded changed-file inventory, and direct source inspection. No commit was created.

## Verdict

- **Outcome:** `pass`
- **Required next state:** `complete`
- **Fix work order required:** `no`
- **Evidence:** The implementation is within WO-001's authorized scale/page-geometry scope. [`raw_input_hook`](../../../../apps/scriptcraft/src/main.rs#L1259-L1299) sanitizes invalid native scale, resolves UI zoom before the frame, replaces fallback when valid native scale arrives, and consumes pinch zoom once. [`resolved_ui_pixels_per_point`](../../../../apps/scriptcraft/src/status_bar.rs#L10-L19) uses the 2.3 fallback only for missing/invalid values and sets zoom to 1.0 for valid native scale. [`presentation_metrics_at_zoom`](../../../../apps/scriptcraft/src/main.rs#L1013-L1027) gives the page a fixed 360-point baseline independent of available width; [`page_scroll_geometry`](../../../../apps/scriptcraft/src/main.rs#L1038-L1056) uses the page plus 16-point buffers and treats equality as fitting. The rendered surface uses ScrollArea's inner available width and zeroes implicit horizontal item spacing ([`live_document_surface`](../../../../apps/scriptcraft/src/main.rs#L535-L578)). Fresh focused verification, `cargo test -p scriptcraft wo001_`, passed all 8 tests, including native-scale transitions/no-double-scaling and exact/rendered scroll-threshold cases.

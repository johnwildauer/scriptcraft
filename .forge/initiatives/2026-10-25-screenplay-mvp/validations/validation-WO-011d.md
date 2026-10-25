# Validation: WO-011d — Native Input Capture Hardening

## Validation metadata

- **Review type:** `validation`
- **Status:** `blocked`
- **Validator:** Independent fresh-context Forge validator
- **Validator session/agent ID:** `validation-session-2026-10-25-wo-011d`
- **Session exception:** none
- **Date:** 2026-10-25
- **Initiative:** [`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:1)
- **Validation file:** `validation-WO-011d.md`
- **Validated work order:** [`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:1)
- **Original work order:** [`WO-011-keyboard-text-input-loop.md`](../work-orders/WO-011-keyboard-text-input-loop.md:1)
- **Compared commit/diff:** Current working-tree source and implementer changed-file/diff evidence; no committed baseline
- **Product source edited by validator:** No

## Comparison basis and process

I read [`00-overview.md`](../../../process/00-overview.md:1), [`05-validation.md`](../../../process/05-validation.md:1), [`project-manifest.md`](../../../project-manifest.md:1), the initiative plan [`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:1), formal review [`design-review-wo-011d-formal.md`](../reviews/design-review-wo-011d-formal.md:1), [`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:1), [`execution-log.md`](../execution-log.md:1), [`keyboard-input-handoff.md`](../keyboard-input-handoff.md:1), relevant requirements/blueprints, current [`main.rs`](../../../apps/scriptcraft/src/main.rs:1), prior WO-011 through WO-011c validation evidence, and sibling WordCraft [`canvas.rs`](../../../../craft/wordcraft/crates/ui-egui/src/canvas.rs:1225), [`keys.rs`](../../../../craft/wordcraft/crates/ui-egui/src/keys.rs:201), and [`main.rs`](../../../../craft/wordcraft/apps/wordcraft/src/main.rs:42).

The comparison is meaningful: the work order is `in-review`, the implementer recorded the complete changed-file list and validation attempts, and current source contains the reported implementation. I did not modify product source or silently repair evidence. The Cargo results below are recorded implementation evidence inspected from [`execution-log.md`](../execution-log.md:40) and [`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:170); they were not rerun in this inspection-only validator tool context.

## Authorized behavior and evidence

| Check | Evidence | Result |
| --- | --- | --- |
| One stable whole-surface response/ID | [`EDITOR_SURFACE_ID`](../../../apps/scriptcraft/src/main.rs:12) is used by the sole enclosing [`ui.interact()`](../../../apps/scriptcraft/src/main.rs:375) over the scroll viewport in [`live_document_surface()`](../../../apps/scriptcraft/src/main.rs:349). [`paint_page()`](../../../apps/scriptcraft/src/main.rs:262) only allocates and paints page geometry. Search found no other `ui.interact()` or `TextEdit` in app source. | **PASS** |
| Pointer-driven activation and retained focus | The enclosing response calls [`response.request_focus()`](../../../apps/scriptcraft/src/main.rs:399) for pointer activity mapped into page geometry, then synchronizes [`surface_focused`](../../../apps/scriptcraft/src/main.rs:411) from [`response.has_focus()`](../../../apps/scriptcraft/src/main.rs:411). The rendered regression starts unfocused, sends pointer events through [`live_document_surface()`](../../../apps/scriptcraft/src/main.rs:942), checks the stable ID, and verifies later-frame focus and dispatch ([`rendered_page_editor_retains_focus_for_later_frame_input()`](../../../apps/scriptcraft/src/main.rs:911)). | **PASS** |
| Explicit egui focus-lock/filter | Focused frames install [`set_focus_lock_filter()`](../../../apps/scriptcraft/src/main.rs:415) for Tab, horizontal/vertical arrows, and Escape; unfocused frames do not install it. This matches the bounded WordCraft canvas pattern ([`canvas.rs`](../../../../craft/wordcraft/crates/ui-egui/src/canvas.rs:1225)). | **PASS** |
| Focused-only event consumption | [`handle_surface_input()`](../../../apps/scriptcraft/src/main.rs:522) returns before cloning or dispatching events when [`surface_focused`](../../../apps/scriptcraft/src/main.rs:524) is false. Focused input reaches [`dispatch_surface_events()`](../../../apps/scriptcraft/src/main.rs:431), preserving text, paste, key, IME, duplicate-commit suppression, and [`EditingSession`](../../../crates/scriptcraft-core/src/editing.rs:59) routing. The unfocused regression asserts no text consumption ([`unfocused_surface_does_not_consume_keyboard_events()`](../../../apps/scriptcraft/src/main.rs:893)). | **PASS** |
| No hidden/duplicate editor or core changes | The app has one [`EditingSession`](../../../apps/scriptcraft/src/main.rs:26), no `TextEdit`, no shadow text buffer, and no alternate editor state. The reported changed-file list contains only the app seam, authorized desktop blueprint, handoff, work order, and execution log; no core, layout, requirements, sibling WordCraft, or unrelated UI files changed. | **PASS** |
| Bounded diagnostics and lifecycle | [`NativeInputDiagnostics`](../../../apps/scriptcraft/src/main.rs:14) uses saturating counters, booleans, and fixed event-category strings only. The status line exposes bounded focus/lock/dispatch state ([`update()`](../../../apps/scriptcraft/src/main.rs:659)); lifecycle is documented as retained status instrumentation in [`keyboard-input-handoff.md`](../keyboard-input-handoff.md:47) and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:43). No document payload or editable buffer is logged. | **PASS** |
| Rendered regressions | The production-seam test covers unfocused start, pointer activation, stable response focus, focus-lock installation, later-frame retention, later-frame text and Backspace dispatch. Additional tests cover Unicode, IME preedit/commit suppression, editing keys, selection, and source-mapped caret behavior ([`tests`](../../../apps/scriptcraft/src/main.rs:792)). | **PASS** |
| Required Cargo gates | The implementer recorded passing [`cargo fmt --all -- --check`](../work-orders/WO-011d-native-input-capture-hardening.md:128), [`cargo check --workspace`](../work-orders/WO-011d-native-input-capture-hardening.md:128), [`cargo build --workspace`](../work-orders/WO-011d-native-input-capture-hardening.md:128), [`cargo test --workspace -- --nocapture`](../work-orders/WO-011d-native-input-capture-hardening.md:128), and [`cargo clippy --workspace --all-targets -- -D warnings`](../work-orders/WO-011d-native-input-capture-hardening.md:128). Recorded result: 38 tests, zero failures, no clippy warnings. | **PASS — recorded evidence** |
| Changed-file/diff evidence | The implementation evidence and execution log agree on the five changed artifacts: [`main.rs`](../../../apps/scriptcraft/src/main.rs:1), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:1), [`keyboard-input-handoff.md`](../keyboard-input-handoff.md:1), [`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:1), and [`execution-log.md`](../execution-log.md:1). The app evidence removes per-page interaction and adds one enclosing interaction. | **PASS** |
| Native macOS smoke | The recorded [`cargo run -p scriptcraft`](../work-orders/WO-011d-native-input-capture-hardening.md:177) launch reached the binary but had no observable window interaction. No native pointer, ASCII/Unicode text, editing-key, IME, caret, or layout-update result is claimed. | **BLOCKED** |

## Findings

### BLOCKER — Required observable native gate is unavailable

- **Severity:** blocker for WO-011d validation/closeout.
- **Evidence:** The work order requires an observable macOS window run showing diagnostics changing for pointer focus and actual native text/key/IME delivery followed by visible document/caret/layout updates ([`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:136)). The execution record states that the bounded launch had no interactively observable window/input sequence and explicitly leaves native success unclaimed ([`execution-log.md`](../execution-log.md:48)). The handoff keeps this discrepancy as a closeout blocker ([`keyboard-input-handoff.md`](../keyboard-input-handoff.md:3)).
- **Recommendation:** Run the real native macOS sequence against the built app: activate the page, type ASCII and Unicode text, exercise Enter/Backspace/Delete/arrows/Shift selection and IME preedit/commit, and observe diagnostics plus visible rerender. Record passed, failed, or unavailable observations without treating rendered egui tests or a terminal launch as native proof.
- **Required next state:** `blocked`; keep [`WO-011d-native-input-capture-hardening.md`](../work-orders/WO-011d-native-input-capture-hardening.md:1) `in-review` and the initiative open until observable native evidence exists or the owner explicitly accepts the documented MVP limitation.

### WARNING — Cargo results are implementation evidence, not independently rerun here

- **Severity:** warning; not a product-source defect.
- **Evidence:** The implementer recorded all required gates as passing ([`execution-log.md`](../execution-log.md:47)), and source/tests are consistent with that report. This validator tool context had repository inspection and patch tools but no command-execution tool, so I did not represent those results as commands independently executed by this session.
- **Recommendation:** Preserve the exact recorded outputs; rerun the focused and full Cargo gates in an execution-capable validation session if independent command execution is required.
- **Required next state:** retain as a validation evidence note; it does not create a fix work order by itself.

## Scope and rogue-behavior assessment

The inspected implementation is aligned with the authorized WO-011d boundary. It preserves the semantic session, source-mapped page hit testing, pointer drag selection, existing event adapter, and page rendering. I found no hidden or duplicate editor, core/model mutation outside the existing session route, sibling WordCraft modification, unrelated UI change, deceptive behavior, backdoor, malicious behavior, or unauthorized product-scope expansion. Requirements are correctly treated as verified unchanged; the desktop blueprint update is within the work order's conditional durable-document authorization.

## Verdict

**BLOCKED — WO-011d implementation alignment passes, but the required observable native macOS acceptance evidence is unavailable.** The stable whole-surface response, pointer-retained focus, focus lock, focused-only dispatch, bounded diagnostics, rendered regressions, scope boundary, changed-file evidence, and recorded Cargo gates pass inspection. Native macOS success remains explicitly unclaimed because no observable window interaction exists. The work order must not move to `complete` on this evidence alone.

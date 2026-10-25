# Validation: WO-011e — Caret Alignment and Editor Focus Polish

## Validation metadata

- **Review type:** `validation`
- **Status:** `blocked`
- **Validator:** Independent fresh-context Forge validator
- **Validator session/agent ID:** `validation-session-2026-10-25-wo-011e`
- **Session exception:** `none`
- **Date:** 2026-10-25
- **Initiative:** [`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:1)
- **Validation file:** `validation-WO-011e.md`
- **Validated work order:** [`WO-011e-caret-and-editor-focus-polish.md`](../work-orders/WO-011e-caret-and-editor-focus-polish.md:1)
- **Compared commit/diff:** Current working-tree source plus the implementer's changed-file record and execution-log evidence; no committed baseline exists.
- **Product source edited by validator:** No

## Comparison basis and process

I read [`00-overview.md`](../../../process/00-overview.md:1), [`05-validation.md`](../../../process/05-validation.md:1), [`project-manifest.md`](../../../project-manifest.md:1), the initiative plan [`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:1), [`WO-011e-caret-and-editor-focus-polish.md`](../work-orders/WO-011e-caret-and-editor-focus-polish.md:1), prior [`validation-WO-011d.md`](validation-WO-011d.md:1), prior WO-011/WO-011a/WO-011b/WO-011c artifacts, the relevant [`execution-log.md`](../execution-log.md:1), current [`main.rs`](../../../apps/scriptcraft/src/main.rs:1), current [`layout.rs`](../../../crates/scriptcraft-core/src/layout.rs:1), and the WordCraft focus/caret/input references [`canvas.rs`](../../../../craft/wordcraft/crates/ui-egui/src/canvas.rs:1316), [`keys.rs`](../../../../craft/wordcraft/crates/ui-egui/src/keys.rs:257), and [`caret_glide.rs`](../../../../craft/wordcraft/crates/ui-egui/src/caret_glide.rs:1).

The work order is `in-review`, its implementation evidence identifies the changed files and validation attempts, and the current source contains the reported implementation. This is a meaningful comparison. The available validator tools did not provide command execution or observable native-window interaction, so recorded Cargo results are identified as implementation evidence rather than independently rerun results. No product source was modified and no issue was silently repaired.

## Authorized scope and evidence

| Check | Evidence | Result |
| --- | --- | --- |
| Fixed-metric glyph/caret alignment without core layout changes | [`paint_fixed_metric_text()`](../../../apps/scriptcraft/src/main.rs:669) places each Unicode scalar at `origin + column * char_width`; [`paint_page()`](../../../apps/scriptcraft/src/main.rs:278) derives `char_width` from [`CHAR_WIDTH`](../../../crates/scriptcraft-core/src/layout.rs:10) and uses the same cell width for selection geometry. [`ScreenplayLayout::caret_rect()`](../../../crates/scriptcraft-core/src/layout.rs:143) remains unchanged and returns `width: 1` at `columns * CHAR_WIDTH`. | **PASS by source inspection and recorded focused regression** |
| Narrow caret geometry | [`CARET_WIDTH`](../../../apps/scriptcraft/src/main.rs:14) is `1.5`; [`caret_bar_rect()`](../../../apps/scriptcraft/src/main.rs:696) uses that width independently of the seven-unit layout cell and uses the source-mapped caret height. The focused [`fixed_metric_rendering_and_caret_bar_do_not_accumulate_drift()`](../../../apps/scriptcraft/src/main.rs:1222) asserts both long-column x alignment and narrow width. | **PASS by source/tests** |
| Focused collapsed-only blink/repaint with no model mutation | [`paint_page()`](../../../apps/scriptcraft/src/main.rs:324) enters the caret path only for a collapsed selection, keys [`animate_bool_with_time()`](../../../apps/scriptcraft/src/main.rs:338) by editor/revision/caret identity, enables it only from `surface_focused`, and requests repaint while focused. [`caret_should_paint()`](../../../apps/scriptcraft/src/main.rs:692) rejects unfocused, non-collapsed, and off phases. The blink phase is not stored in [`ScriptCraftApp`](../../../apps/scriptcraft/src/main.rs:27) or [`EditingSession`](../../../crates/scriptcraft-core/src/editing.rs:59); the focused [`caret_blink_is_focused_collapsed_and_transient()`](../../../apps/scriptcraft/src/main.rs:1247) test covers gating. | **PASS by source/tests; native visual blink remains unclaimed** |
| Blank-space pointer focus activation | The sole enclosing response in [`live_document_surface()`](../../../apps/scriptcraft/src/main.rs:391) requests focus before optional page hit testing in [`response.request_focus()`](../../../apps/scriptcraft/src/main.rs:402). A miss leaves selection/caret unchanged but still activates the response. The rendered [`rendered_page_editor_retains_focus_for_later_frame_input()`](../../../apps/scriptcraft/src/main.rs:972) drives blank enclosing-surface space and asserts focus/activation. | **PASS for rendered path** |
| Retained ordinary key input | [`surface_focused`](../../../apps/scriptcraft/src/main.rs:431) is synchronized from the stable response and [`handle_surface_input()`](../../../apps/scriptcraft/src/main.rs:542) dispatches only while focused. The rendered regression inserts later text and Backspace after blank-space activation, while the existing Unicode and editing-key tests remain present. | **PASS for rendered/headless path; native printable-key result unclaimed** |
| Toolbar/dialog focus transfer | Toolbar controls are rendered separately in [`ScriptCraftApp::update()`](../../../apps/scriptcraft/src/main.rs:721); editor ownership is derived from [`response.has_focus()`](../../../apps/scriptcraft/src/main.rs:431), and event dispatch returns when the editor is not focused. This preserves the WO-011d response-owned transfer boundary and does not add a per-page response. No native dialog interaction was observable here. | **PASS by source alignment; native transfer unclaimed** |
| No duplicate editor or core layout changes | Source inspection found one [`EditingSession`](../../../apps/scriptcraft/src/main.rs:27), no `TextEdit`, no shadow text buffer, and one enclosing editor response. [`layout.rs`](../../../crates/scriptcraft-core/src/layout.rs:1) retains `CHAR_WIDTH`, wrapping, hit testing, and caret geometry. The implementation changed-file record names only [`main.rs`](../../../apps/scriptcraft/src/main.rs:1), the WO, and [`execution-log.md`](../execution-log.md:1); no core/layout, persistence, exports, IME, diagnostics, blueprint, requirements, or WordCraft files were changed. | **PASS** |
| Prior selection/input behavior retained | [`apply_pointer_selection()`](../../../apps/scriptcraft/src/main.rs:217) retains drag-anchor behavior and valid source-mapped hits; existing app tests cover drag selection, Unicode, IME commit suppression, editing keys, role transition, undo/redo, source mapping, and single-surface behavior. | **PASS by source and recorded suite** |
| Required Cargo gates | The implementer recorded `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo build --workspace`, `cargo test --workspace -- --nocapture` with 15 app, 22 core, and 3 integration tests plus zero doc-test failures, and `cargo clippy --workspace --all-targets -- -D warnings` with no warnings. | **PASS — recorded implementation evidence; not independently rerun in this inspection-only context** |
| Changed-file/diff evidence | The WO evidence and [`execution-log.md`](../execution-log.md:73) agree on the three implementation artifacts and describe a bounded app-boundary diff: explicit glyph placement, narrow animation caret, and enclosing-response focus ordering. No unrelated or rogue behavior was identified. | **PASS** |
| Native macOS smoke | The recorded [`cargo run -p scriptcraft`](../execution-log.md:83) reached the binary and was stopped at the five-second bound with `bounded_smoke_exit=143`. No interactive window was observable. Therefore native caret alignment, blink, blank-space activation, continued typing, selection, and toolbar/dialog transfer are not claimed. | **BLOCKED** |

## Findings

### Blocker — required native/manual acceptance evidence is unavailable

- **Severity:** blocker for WO-011e completion and initiative closeout.
- **Evidence:** [`WO-011e-caret-and-editor-focus-polish.md`](../work-orders/WO-011e-caret-and-editor-focus-polish.md:104) explicitly requires an observable native macOS smoke sequence covering long-line typing, blank-space activation, continued typing, blink, narrow caret appearance, selection, and toolbar focus transfer, and forbids claiming native success from synthetic tests or a bounded launch. The implementation record states that no interactive window was observable and explicitly leaves every native result unclaimed ([`execution-log.md`](../execution-log.md:83), [`WO-011e-caret-and-editor-focus-polish.md`](../work-orders/WO-011e-caret-and-editor-focus-polish.md:136)). The prior WO-011d validation already preserves the native closeout blocker ([`validation-WO-011d.md`](validation-WO-011d.md:41)).
- **Recommendation:** Run the required observable macOS sequence against the current binary and record each result: long-line caret alignment, narrow/blinking caret, blank-space activation, ordinary typing without a second precise line click, selection, and toolbar/dialog/native-file-dialog focus transfer. Do not substitute rendered egui tests or a terminal launch for native evidence.
- **Required next state:** `blocked`; keep WO-011e `in-review` and preserve the initiative blocker until native evidence is available or an authorized owner decision explicitly accepts the documented limitation.

### Warning — recorded Cargo gates were not independently rerun

- **Severity:** warning, not a product-source defect.
- **Evidence:** The implementer recorded all required commands and results in [`WO-011e-caret-and-editor-focus-polish.md`](../work-orders/WO-011e-caret-and-editor-focus-polish.md:135) and [`execution-log.md`](../execution-log.md:82). This validator context provided repository inspection and patch tools but no command-execution tool.
- **Recommendation:** Preserve the recorded outputs; rerun the focused and full Cargo gates in an execution-capable validation session if independent command execution is required.
- **Required next state:** retain as an evidence note; it does not create a fix work order.

## Scope and rogue-behavior assessment

The inspected code aligns with the authorized WO-011e boundary. It keeps fixed metrics and source-mapped caret geometry in the existing core contract, confines glyph/caret presentation and focus ordering to the existing desktop surface, preserves the one-editor architecture and existing session mutation path, and does not change persistence, exports, IME semantics, diagnostics lifecycle, requirements, layout algorithms, or sibling WordCraft code. I found no clearly unrelated, deceptive, backdoor, malicious, nefarious, or other rogue behavior.

## Decision

- **Outcome:** `blocked`
- **Required next state:** Keep WO-011e `in-review`; keep the initiative `blocked` pending the required observable native smoke evidence or an authorized documented acceptance of its unavailability.
- **Fix work order required:** `no` based on the available code alignment. The blocker is an unavailable required external/native evidence gate, not a demonstrated implementation defect. A later native result may establish pass or reveal a separate implementation finding.

**BLOCKED — WO-011e code alignment passes inspection, but completion cannot be claimed because the required native interaction evidence is unavailable.**

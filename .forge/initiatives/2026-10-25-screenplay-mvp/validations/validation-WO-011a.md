# Validation: WO-011a — Retain Page Editor Focus

## Validation metadata

- **Review type:** `validation`
- **Status:** `fail`
- **Validator:** Independent fresh-context validator
- **Validator session/agent ID:** `validation-session-2026-10-25-wo-011a`
- **Session exception:** `none`
- **Date:** 2026-10-25
- **Initiative:** [`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:1)
- **Validation file:** `validation-WO-011a.md`
- **Validated work order:** [`WO-011a-retain-page-editor-focus.md`](../work-orders/WO-011a-retain-page-editor-focus.md:1)
- **Original work order:** [`WO-011-keyboard-text-input-loop.md`](../work-orders/WO-011-keyboard-text-input-loop.md:1)
- **Compared commit/diff:** Current source and working-tree diff; no committed baseline.
- **Product source edited:** No

## Comparison basis and process

I read [`05-validation.md`](../../../process/05-validation.md:1), the failing [`validation-WO-011.md`](validation-WO-011.md:1), [`WO-011a-retain-page-editor-focus.md`](../work-orders/WO-011a-retain-page-editor-focus.md:1), [`WO-011-keyboard-text-input-loop.md`](../work-orders/WO-011-keyboard-text-input-loop.md:1), the relevant [`execution-log.md`](../execution-log.md:1), and current [`main.rs`](../../../apps/scriptcraft/src/main.rs:1). I independently inspected the reported focus fix and regression, ran the required Cargo gates, ran the focused regression, and performed a bounded native launch smoke.

## Diagnostic hypotheses and evidence log

The possible sources considered were: (1) the reported stable egui response identity was not implemented in the rendered path; (2) focus was still reset before later-frame dispatch; (3) pointer activation still did not synchronize from `response.has_focus()`; (4) the regression only modeled focus in isolation rather than exercising the app surface; (5) the later-frame event dispatcher remained disconnected from the real painted response; (6) text/IME dispatch or duplicate suppression regressed; and (7) build/test/toolchain or native launch behavior invalidated the evidence. The first four reduced to the likely hard-stop cause; the event and quality paths passed independently.

| Diagnostic check | Evidence | Result |
| --- | --- | --- |
| Stable rendered response identity | [`paint_page()`](../../../apps/scriptcraft/src/main.rs:237) allocates its response directly with `ui.allocate_exact_size(...)`; there is no `ui.push_id("page-editor", ...)` or equivalent stable page-editor scope in the production rendering path. | **FAIL** |
| Focus retention/reset | [`live_document_surface()`](../../../apps/scriptcraft/src/main.rs:345) still assigns `self.surface_focused = false` on every render. | **FAIL / blocker** |
| Pointer focus synchronization | [`paint_page()`](../../../apps/scriptcraft/src/main.rs:252) calls `response.request_focus()` and sets the flag only in the pointer-hit branch; it never assigns the flag from `response.has_focus()` each frame. | **FAIL** |
| Later-frame production dispatch | [`handle_surface_input()`](../../../apps/scriptcraft/src/main.rs:446) gates dispatch on the flag that the production surface clears. The rendered app path therefore still drops later-frame keyboard/IME input. | **FAIL** |
| Regression relevance | [`page_editor_retains_focus_for_later_frame_input()`](../../../apps/scriptcraft/src/main.rs:796) creates a separate synthetic response twice, requests focus manually, then manually assigns `app.surface_focused = second_frame_focused` before calling [`dispatch_surface_events()`](../../../apps/scriptcraft/src/main.rs:366). It does not invoke [`live_document_surface()`](../../../apps/scriptcraft/src/main.rs:345), [`paint_page()`](../../../apps/scriptcraft/src/main.rs:237), or [`handle_surface_input()`](../../../apps/scriptcraft/src/main.rs:446). | **FAIL / blocker** |
| Text and key dispatch | The focused test and full workspace tests pass; [`dispatch_surface_events()`](../../../apps/scriptcraft/src/main.rs:366) still routes text and editing keys through [`EditingSession`](../../../crates/scriptcraft-core/src/editing.rs:59). | **PASS in isolation; does not cure focus blocker** |
| IME behavior | Existing IME preedit/commit duplicate-suppression test passes. | **PASS in isolation** |

## Independent command evidence

- `cargo fmt --all -- --check` — **PASS**.
- `cargo check --workspace` — **PASS**.
- `cargo build --workspace` — **PASS**.
- `cargo test --workspace -- --nocapture` — **PASS**: 12 app tests, 22 core tests, 3 WO-008 integration tests, and doc-test target; zero failures.
- Focused `page_editor_retains_focus_for_later_frame_input` test — **PASS**, but the test's manual flag assignment and synthetic response path make it insufficient evidence for the required app-level acceptance criterion.
- `cargo clippy --workspace --all-targets -- -D warnings` — **PASS**.
- Bounded `cargo run -p scriptcraft` — **PASS as launch smoke only**: reached `target/debug/scriptcraft`, emitted no window/input interaction evidence, and was terminated after five seconds with `bounded_smoke_exit=143`. No visual/manual success is claimed.

## Finding

### Blocker — WO-011a did not reach the production page-editor focus path

The claimed fix is not present in the current rendered application source. The prior hard stop remains intact: production rendering has no stable page-editor ID scope, clears `surface_focused` at the start of every surface render, and does not synchronize it from retained `response.has_focus()`. The regression passes only because it bypasses those methods and directly sets the app-side flag after a synthetic egui exercise. Consequently it does not prove that pointer activation admits text/key events in the later frame through the real app path.

This is a plainly unmet required criterion in [`WO-011a-retain-page-editor-focus.md`](../work-orders/WO-011a-retain-page-editor-focus.md:25), not a visual-evidence limitation. No unrelated, deceptive, backdoor, malicious, or rogue behavior was found.

## Required next state

Create and dispatch the next lettered fix work order **WO-011b — wire stable page-editor focus into the rendered path**, followed by a fresh implementation and validation session. The fix must add the stable ID scope and retained `response.has_focus()` synchronization to the actual [`live_document_surface()`](../../../apps/scriptcraft/src/main.rs:345)/[`paint_page()`](../../../apps/scriptcraft/src/main.rs:237) path, remove the per-frame reset, preserve toolbar/dialog focus ownership, and revise the regression so it drives the production surface and later-frame [`handle_surface_input()`](../../../apps/scriptcraft/src/main.rs:446) dispatch rather than manually setting `surface_focused`. Do not create or claim completion for WO-011b in this validation session.

## Verdict

**FAIL — WO-011a remains `in-review`; WO-011 remains `in-review`.** The quality gates, isolated text/IME/key dispatch, and bounded launch smoke pass, but the implementation does not resolve the original focus/capture hard stop. The fix chain must continue with WO-011b.

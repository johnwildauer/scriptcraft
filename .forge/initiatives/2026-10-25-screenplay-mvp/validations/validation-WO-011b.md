# Validation: WO-011b — Wire Rendered Page Editor Focus

## Validation metadata

- **Review type:** `validation`
- **Status:** `fail`
- **Validator:** Independent fresh-context validator
- **Validator session/agent ID:** `validation-session-2026-10-25-wo-011b`
- **Session exception:** `none`
- **Date:** 2026-10-25
- **Initiative:** [`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:1)
- **Validation file:** `validation-WO-011b.md`
- **Validated work order:** [`WO-011b-wire-rendered-page-editor-focus.md`](../work-orders/WO-011b-wire-rendered-page-editor-focus.md:1)
- **Prior failing validation:** [`validation-WO-011a.md`](validation-WO-011a.md:1)
- **Original work order:** [`WO-011-keyboard-text-input-loop.md`](../work-orders/WO-011-keyboard-text-input-loop.md:1)
- **Compared commit/diff:** Current source and working-tree diff; no committed baseline.
- **Product source edited:** No

## Comparison basis and process

I read [`05-validation.md`](../../../process/05-validation.md:1), the failing [`validation-WO-011a.md`](validation-WO-011a.md:1), [`WO-011b-wire-rendered-page-editor-focus.md`](../work-orders/WO-011b-wire-rendered-page-editor-focus.md:1), [`WO-011a-retain-page-editor-focus.md`](../work-orders/WO-011a-retain-page-editor-focus.md:1), [`WO-011-keyboard-text-input-loop.md`](../work-orders/WO-011-keyboard-text-input-loop.md:1), the relevant [`execution-log.md`](../execution-log.md:1), and current [`main.rs`](../../../apps/scriptcraft/src/main.rs:1). I independently inspected the rendered focus seam and regression, ran the focused and workspace tests, ran all required Cargo gates, and performed a bounded native launch smoke.

## Diagnostic hypotheses and evidence log

The possible sources considered were: (1) the stable page-editor identity was still absent from production rendering; (2) the rendered response did not request focus on pointer activation; (3) the application dispatch gate was still reset or not synchronized from retained response focus; (4) the regression bypassed the production surface or later-frame handler; (5) the regression seeded focus before pointer activation and therefore could pass without testing the request path; (6) later-frame text/key dispatch or IME behavior regressed; and (7) formatting, compilation, test, lint, or native launch behavior invalidated the evidence. Sources (1)–(4) are resolved in the current implementation; source (5) is the remaining hard stop. Sources (6)–(7) passed independently.

| Diagnostic check | Evidence | Result |
| --- | --- | --- |
| Stable rendered response identity | [`paint_page()`](../../../apps/scriptcraft/src/main.rs:237) reserves the page rectangle, creates deterministic `egui::Id::new(("page-editor", page_index))` at line 248, and obtains the interaction response through [`ui.interact()`](../../../apps/scriptcraft/src/main.rs:249). The same ID is used by the rendered page on later frames. | **PASS** |
| Pointer focus request | The rendered pointer path calls [`response.request_focus()`](../../../apps/scriptcraft/src/main.rs:270) after a valid hit/selection activation. | **PASS by source inspection; not proven by the regression** |
| Per-frame focus synchronization | [`paint_page()`](../../../apps/scriptcraft/src/main.rs:277) ORs [`response.has_focus()`](../../../apps/scriptcraft/src/main.rs:277) into the page aggregate, and [`live_document_surface()`](../../../apps/scriptcraft/src/main.rs:362) assigns the aggregate after rendering without the former per-frame reset. | **PASS** |
| Later-frame production dispatch | [`handle_surface_input()`](../../../apps/scriptcraft/src/main.rs:459) gates on `surface_focused`; the regression invokes [`live_document_surface()`](../../../apps/scriptcraft/src/main.rs:350) and [`handle_surface_input()`](../../../apps/scriptcraft/src/main.rs:459) in later frames and observes text plus Backspace. | **PASS for the exercised seeded-focus path** |
| Regression proves pointer activation | Before the pointer press, the test directly calls [`memory.request_focus()`](../../../apps/scriptcraft/src/main.rs:842) for the page ID. The subsequent pointer frame therefore begins with focus already queued; the test does not establish that the pointer activation itself caused focus. | **FAIL / blocker** |
| Text/key and IME dispatch | The focused rendered test and full workspace tests pass; the existing [`dispatch_surface_events()`](../../../apps/scriptcraft/src/main.rs:379) path still routes text and editing keys through [`EditingSession`](../../../crates/scriptcraft-core/src/editing.rs:5). Existing IME duplicate-suppression coverage passes. | **PASS in the tested paths** |
| Quality and launch evidence | The required Cargo gates pass. Bounded launch reaches `target/debug/scriptcraft` and is terminated at the bound; no visual/window interaction evidence is claimed. | **PASS as bounded launch smoke; visual evidence unavailable** |

## Independent command evidence

- `cargo test -p scriptcraft rendered_page_editor_retains_focus_for_later_frame_input -- --nocapture` — **PASS**: 1 focused test.
- `cargo fmt --all -- --check` — **PASS**.
- `cargo check --workspace` — **PASS**.
- `cargo build --workspace` — **PASS**.
- `cargo test --workspace -- --nocapture` — **PASS**: 12 app tests, 22 core tests, 3 WO-008 integration tests, and doc-test target; zero failures.
- `cargo clippy --workspace --all-targets -- -D warnings` — **PASS**.
- Bounded `cargo run -p scriptcraft` — **PASS as launch smoke only**: reached `target/debug/scriptcraft`, emitted no window/input interaction evidence, and was terminated after five seconds with `bounded_smoke_exit=143`. No visual/manual success is claimed.

## Finding

### Blocker — the rendered regression still seeds focus synthetically

WO-011b did wire the actual rendered path: the stable ID/response, [`ui.interact()`](../../../apps/scriptcraft/src/main.rs:249), pointer focus request, per-frame [`response.has_focus()`](../../../apps/scriptcraft/src/main.rs:277) synchronization, and later-frame [`handle_surface_input()`](../../../apps/scriptcraft/src/main.rs:459) dispatch are present. However, [`rendered_page_editor_retains_focus_for_later_frame_input()`](../../../apps/scriptcraft/src/main.rs:809) calls [`context.memory_mut(... request_focus(...))`](../../../apps/scriptcraft/src/main.rs:842) before the pointer press. This makes the focus assertion and later-frame text/Backspace result compatible with a test that would pass even if the pointer activation's [`response.request_focus()`](../../../apps/scriptcraft/src/main.rs:270) were removed. It is therefore still synthetic-only for the critical pointer-to-focus edge, despite exercising the production render and handler methods afterward.

This is a plainly unmet acceptance requirement in [`WO-011b-wire-rendered-page-editor-focus.md`](../work-orders/WO-011b-wire-rendered-page-editor-focus.md:75): the regression must prove that rendered pointer activation requests focus, then that a later rendered frame reports focus and dispatches input. No unrelated, deceptive, backdoor, malicious, or rogue behavior was found.

## Required next state

Create and dispatch the next lettered fix work order **WO-011c — prove pointer-driven rendered focus activation**, followed by a fresh implementation and validation session. The fix must keep the current production wiring, remove the direct pre-activation [`memory.request_focus()`](../../../apps/scriptcraft/src/main.rs:842) from the regression, drive a genuine egui pointer press/release activation over the stable page response, and assert that focus is absent before activation, requested by the activation frame, retained in the later frame, and sufficient for later-frame text plus an editing key through [`handle_surface_input()`](../../../apps/scriptcraft/src/main.rs:459). Do not edit product code or create that fix work order in this validation session.

## Verdict

**FAIL — WO-011b remains `in-review`; WO-011a remains `in-review`; WO-011 remains `in-review`.** The original production wiring hard stop is resolved, and all quality gates plus bounded smoke pass, but the required regression is not independent of the focus request it claims to verify. The fix chain must continue with WO-011c.

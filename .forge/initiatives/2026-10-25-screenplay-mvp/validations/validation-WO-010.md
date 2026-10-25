# Validation: WO-010 — Basic Named Persistence UI

## Validation metadata

- **Review type:** `validation`
- **Status:** `pass`
- **Validator:** Independent fresh-context validator
- **Validator session/agent ID:** `validation-session-2026-10-25-wo-010`
- **Session exception:** `none`
- **Date:** 2026-10-25
- **Initiative:** [`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md:1)
- **Validation file:** `validation-WO-010.md`
- **Validated work order:** [`WO-010-basic-named-persistence-ui.md`](../work-orders/WO-010-basic-named-persistence-ui.md:1)
- **Compared commit/diff:** No committed baseline; comparison used the recorded changed-file evidence and current source.

## Authorized scope and comparison basis

I read the validation process, [`pre-closeout-test-checklist.md`](../pre-closeout-test-checklist.md:1), WO-010, the scope-correction note, the UI-correction design review, the execution log, the WO-009/WO-009a evidence, [`main.rs`](../../../apps/scriptcraft/src/main.rs:1), [`Cargo.toml`](../../../apps/scriptcraft/Cargo.toml:1), and core [`persistence.rs`](../../../crates/scriptcraft-core/src/persistence.rs:1). The implementation comparison used the recorded WO-010 changed-file evidence and current source. No product source was edited.

The validation boundary was named native Open, Save, and Save As path ownership and status behavior, dirty/open safety, non-destructive failures, absence of recovery UI and stale messaging, and preservation of the unified WO-009 editor. Recovery discovery/rotation and native persistence contracts were checked as unchanged core boundaries, not revalidated as new UI behavior.

## Findings and independent evidence

### Named Open, Save, Save As, and path state

- **PASS.** [`ScriptCraftApp::open_native()`](../../../apps/scriptcraft/src/main.rs:57) uses a native `rfd::FileDialog` Open selection and delegates the selected path to [`ScriptCraftApp::open_native_at()`](../../../apps/scriptcraft/src/main.rs:66).
- **PASS.** [`ScriptCraftApp::save_native()`](../../../apps/scriptcraft/src/main.rs:37) uses the current optional `native_path` when present and falls back to [`ScriptCraftApp::save_as_native()`](../../../apps/scriptcraft/src/main.rs:89) when no path exists. Save As uses a native dialog and routes the selected path through [`ScriptCraftApp::save_native_at()`](../../../apps/scriptcraft/src/main.rs:45).
- **PASS.** Successful atomic save calls [`EditingSession::mark_saved()`](../../../crates/scriptcraft-core/src/editing.rs:171), then updates `native_path` and status; successful open updates the path only after read, deserialize, and [`EditingSession::replace_with_saved()`](../../../crates/scriptcraft-core/src/editing.rs:181) succeed.
- **PASS.** The focused [`named_save_and_open_update_path_and_clean_state()`](../../../apps/scriptcraft/src/main.rs:678) test independently passed and verified selected-path save/open routing, path replacement, loaded content, and clean state. The original path is not reused after the successful open of the second selected path, and Save As therefore routes through the same atomic-save path without mutating the first file.
- **Note.** Native dialog interaction itself is not observable in this terminal-only environment; the app-local dialog boundary and selected-path adapters are directly inspectable, and the focused path tests exercise the non-dialog seam.

### Dirty/open safety and failure preservation

- **PASS.** [`ScriptCraftApp::open_native_at()`](../../../apps/scriptcraft/src/main.rs:66) refuses to replace a dirty session, clears stale error presentation, and reports `Open cancelled: save the unsaved project first` without changing the session or path.
- **PASS.** [`ScriptCraftApp::show_error()`](../../../apps/scriptcraft/src/main.rs:130) reports `Operation failed; document unchanged`; failed read/deserialize and failed atomic save do not update the named path or mark the session clean.
- **PASS.** The focused [`failed_named_save_preserves_path_file_and_dirty_state()`](../../../apps/scriptcraft/src/main.rs:709) test independently passed and verified that a failed destination preserves the prior path, prior file bytes, dirty state, and visible error state.
- **PASS.** The focused [`failed_open_does_not_replace_named_project()`](../../../apps/scriptcraft/src/main.rs:735) test independently passed and verified that an unreadable selected path preserves the existing path, project text, clean state, and error state. The dirty-open refusal is also explicit in the source guard above; no destructive replacement path exists before that guard.
- **PASS.** Core [`atomic_save()`](../../../crates/scriptcraft-core/src/persistence.rs:72) retains the prior destination when temporary-file rename fails, and the independent workspace persistence tests passed.

### Recovery UI, stale messaging, and scope

- **PASS.** A source search of [`apps/scriptcraft/src`](../../../apps/scriptcraft/src:1) found no recovery action, recovery dialog, recovery candidate presentation, or recovery UI label. Recovery remains exposed only by core [`discover_recovery()`](../../../crates/scriptcraft-core/src/persistence.rs:126), consistent with WO-010's exclusion.
- **PASS.** The same source search found no fixed `scriptcraft.json` UI path and no stale `Persistence host boundary` message. The current status presentation reports the selected/current path or `(unsaved)` at [`ScriptCraftApp::update()`](../../../apps/scriptcraft/src/main.rs:454).
- **PASS.** The manifest adds only the app-local native dialog dependency [`rfd`](../../../apps/scriptcraft/Cargo.toml:8); core persistence/schema/recovery files were not changed by WO-010.

### Unified WO-009 editor remains intact

- **PASS.** [`ScriptCraftApp::live_document_surface()`](../../../apps/scriptcraft/src/main.rs:324) remains the sole central editing surface, with page layout and source-mapped pointer handling in [`ScriptCraftApp::paint_page()`](../../../apps/scriptcraft/src/main.rs:226). The toolbar contains named persistence actions but no Continuous/Page mode switch, duplicate per-element `TextEdit`, or recovery control.
- **PASS.** [`ScriptCraftApp::handle_surface_input()`](../../../apps/scriptcraft/src/main.rs:343) continues routing text, deletion, backspace, Enter, caret movement, undo, and redo through [`EditingSession`](../../../crates/scriptcraft-core/src/editing.rs:59). The existing five WO-009/WO-009a-focused app tests, including [`pointer_drag_preserves_initial_hit_as_selection_anchor()`](../../../apps/scriptcraft/src/main.rs:584), remained green as part of the eight-test app run.

### Required quality gates and bounded launch

- `cargo fmt --all -- --check` — PASS.
- `cargo check --workspace` — PASS.
- `cargo build --workspace` — PASS.
- `cargo test --workspace -- --nocapture` — PASS: 8 app tests, 22 core unit tests, 3 WO-008 integration tests, and doc-test target with zero failures.
- `cargo clippy --workspace --all-targets -- -D warnings` — PASS.
- Focused named-save/open tests — PASS: each of the three named tests passed independently (1/1).
- Bounded `cargo run -p scriptcraft` — reached `target/debug/scriptcraft` and was terminated after five seconds with exit 143. No visible-window or manual interaction result is claimed; project policy does not configure visible-window smoke as a mandatory validation gate.
- The initial combined command attempted an unsupported multi-filter Cargo test invocation and therefore exited before clippy; this was corrected by rerunning clippy and each focused test separately. The final gates above are passing results.

## Blockers

- None. No required implementation criterion failed, and no clearly unrelated, deceptive, backdoor, malicious, nefarious, or rogue behavior was found.

## Warnings

- **Headless visual smoke — Severity: warning.** Terminal-only bounded launch cannot prove native dialog appearance or visual interaction. The source seams and focused tests provide the available evidence; no project-policy manual gate makes this a hard stop.
- **No committed baseline — Severity: warning.** Comparison used current source, the changed-file record, tests, and execution evidence because the branch has no committed baseline.

## Decision

- **Outcome:** `pass`
- **Required next state:** WO-010 `complete`.
- **Fix work order required:** `no`

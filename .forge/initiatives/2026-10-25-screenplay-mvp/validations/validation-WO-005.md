# Validation: WO-005 — Focused Desktop UI

- **Review type:** `validation`
- **Status:** `complete`
- **Validator:** Independent fresh-context validation agent
- **Validator session/agent ID:** `validation-session-2026-10-25-wo-005`
- **Session exception:** `none`
- **Initiative:** [`2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md)
- **Validation file:** `validation-WO-005.md`
- **Validated work order:** [`WO-005-focused-desktop-ui.md`](../work-orders/WO-005-focused-desktop-ui.md)
- **Compared commit/diff:** No commit exists; the checkout is an unborn, fully uncommitted branch. Comparison therefore used the WO, execution log, current source, manifest/dependency graph, and fresh command results. No product source was changed.

## Scope and comparison basis

I read the validation process, initiative context, WO-005, its execution evidence,
the desktop container blueprint, the writing-loop blueprint, [`apps/scriptcraft/Cargo.toml`](../../../apps/scriptcraft/Cargo.toml), and [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs). The reported changed files are the app manifest, lockfile, app entry point, WO, and execution log, all of which are within the authorized focused desktop UI boundary.

The implementation provides one active [`EditingSession`](../../../apps/scriptcraft/src/main.rs:16) as the canonical app model/session path. The editor string is a transient adapter value; replacement, role changes, Enter transitions, undo, redo, selection, validation, dirty state, and revision behavior route through the core session. Page preview calls [`ScreenplayLayout::build`](../../../apps/scriptcraft/src/main.rs:148) and paints returned pages/lines rather than implementing a second paginator.

## Independent checks

All checks below were run independently in this fresh validation session:

| Check | Result | Evidence |
|---|---|---|
| `cargo fmt --all -- --check` | PASS | Exit code 0 |
| `cargo check --workspace` | PASS | Exit code 0 |
| `cargo build --workspace` | PASS | Exit code 0 |
| `cargo test --workspace -- --nocapture` | PASS | 2 app tests, 15 core tests, 0 failures; doc tests passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS | Exit code 0 |
| Focused app test behavior | PASS | New-project initialization and session-routed replacement/dirty/undo tests passed |
| Dependency/scope probe | PASS | [`eframe`](../../../apps/scriptcraft/Cargo.toml:9) is app-local; source inspection found no filesystem save/open, export, or parallel canonical buffer implementation |
| Bounded native launch | ATTEMPTED | `cargo run -p scriptcraft` built and started `target/debug/scriptcraft`; the 5-second bounded probe terminated it with shell exit 142 (`SIGALRM`). No visual success is claimed because no window was observed through a UI automation/screenshot path. |

## Acceptance alignment

- **Valid starting element/title metadata:** [`ScriptCraftApp::new()`](../../../apps/scriptcraft/src/main.rs:26) constructs `Project::new(...)`, creates an [`EditingSession`](../../../apps/scriptcraft/src/main.rs:29), and the focused test confirms one editable Action element with empty text.
- **Role/action controls and transitions:** The UI exposes all seven roles through [`role_controls()`](../../../apps/scriptcraft/src/main.rs:102), and an explicit `Enter → next role` action invokes [`EditingSession::enter()`](../../../apps/scriptcraft/src/main.rs:238). Role changes and transitions use the core mutation result path.
- **Undo/redo:** Toolbar actions and command-key handlers invoke session [`undo()`](../../../apps/scriptcraft/src/main.rs:193) and [`redo()`](../../../apps/scriptcraft/src/main.rs:199), with capability-gated buttons and editor synchronization.
- **Mode toggle and layout display/status:** Continuous and Page preview modes are explicit [`ViewMode`](../../../apps/scriptcraft/src/main.rs:9) values and toolbar controls. Page mode consumes core layout records; the status bar reports edit status, errors, memory-only dirty state, and the persistence boundary.
- **No persistence/export added:** The inspected app source and manifest contain no filesystem persistence or PDF/TXT export implementation. The status text explicitly says `Persistence host boundary: not connected` at [`main.rs`](../../../apps/scriptcraft/src/main.rs:265), consistent with WO-005 exclusions.
- **Headless-capable core preserved:** The UI dependency is confined to the app manifest; the reusable core remains the semantic/session/layout test target, and all 15 core tests pass.

## Diagnostic reflection

Possible sources of a WO-005 failure considered before narrowing were: (1) a broken Cargo dependency or platform backend, (2) formatting/check/build/lint regressions, (3) app tests bypassing the core session, (4) a second authoritative text/model buffer, (5) role/undo/redo controls not being wired, (6) page preview independently paginating or mutating text, and (7) accidental persistence/export scope creep or a launch-only failure.

The two most likely risks were (a) UI adapter mutations bypassing [`EditingSession`](../../../crates/scriptcraft-core/src/editing.rs:59), and (b) the native window path compiling but failing to launch visibly. The focused tests, source probes, and passing workspace gates validate (a). The bounded launch validated process startup/build, but did not establish visual success; the timeout is recorded as a warning rather than treated as a visual observation.

No implementation defect requiring a fix was diagnosed. Consequently, there is no fix to apply and no user confirmation-before-fix step is applicable to this validation-only session.

## Findings

- **Warnings:** The checkout has no commit/diff baseline, so comparison is artifact/source based. The bounded launch was terminated after five seconds and did not provide an observed visual window; no visual claim is made. The project policy does not configure a required manual desktop gate, and the WO explicitly permits desktop smoke evidence where feasible.
- **Notes:** The desktop blueprint remains a desired/draft document with older foundation-era wording about no selected UI framework; WO-005 supplies the authorized concrete `eframe` decision in its own evidence. This documentation lag does not contradict the implementation or prevent comparison.
- **Rogue/unrelated behavior:** None found.
- **Hard-stop findings:** None.

## Outcome

**PASS.** WO-005 is aligned with its authorized focused desktop UI scope. The required next state is to set WO-005 to `complete`; no lettered fix work order is required.

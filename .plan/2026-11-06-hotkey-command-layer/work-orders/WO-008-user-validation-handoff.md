# WO-008 — User-owned validation handoff

- **Status:** Ready for user validation; no results recorded
- **Initiative:** [Hotkeys and consolidated command layer v1](../hotkey-command-layer-v1.md)
- **Implementation record:** [WO-008 durable contracts and validation](WO-008-durable-contracts-and-validation.md)
- **Validation owner:** User. Agents must not run checks, tests, builds, formatting, review, or manual verification.
- **Purpose:** Executable user handoff and blank evidence record, not validation evidence or an acceptance claim.

## How to record evidence

For each scenario below, record the date, checkout/branch, OS and version, app/build profile where
relevant, exact inputs and steps, expected result, observed result, evidence location, and one of
`Pass`, `Fail`, `Blocked`, or `Not run`. Record failures and their impact without silently changing
expected behavior. Leave platform-specific cases `Not run` when the platform is unavailable. Keep
automated command output and manual/native interaction evidence separate. Do not infer a pass from
authored coverage, source routing, another OS, or an effect-dispatched result.

No checks or scenarios have been run for this handoff. Results remain user-owned and blank.

## Validation matrix

| ID | Area | User-owned scenarios | Evidence / outcome |
|---|---|---|---|
| V1 | Command wire contract | Inspect and exercise every one of the 41 stable IDs; valid round-trip envelope and origin preservation; unknown ID; unknown wire version; malformed/missing/wrongly typed/out-of-range payloads; all 17 Role values; Unicode and control-containing text; direct and picker paths; invalid/non-UTF-8 paths; valid, reversed, out-of-range and non-UTF-8-boundary Selections; pointer phase, navigation enums, zoom percent/factor, appearance, timer mode/minutes and import-resolution enums. Confirm no command, origin or transient view value is written to FDX. | **Outcome:** Not run. **Evidence:** |
| V2 | Headless semantic executor | Run the command-crate scenarios and inspect `execute_editor_command()` over a real `EditingSession`: typing, paste, both deletion directions, Enter/Tab for every Role, all 17 explicit Role changes, Select All, Selection updates, Undo/Redo, and unsupported commands. Compare complete observable session state for accepted no-op, unavailable history/selection, malformed Selection, invalid UTF-8, cross-element Role rejection, preservation rejection, capacity failure and rollback. Verify one-step undo/redo for content and role operations. | **Outcome:** Not run. **Evidence:** |
| V3 | Hotkey resolver and focused input | For both explicit logical platforms, check all 17 base/extended bindings, exact modifier sets, wrong-platform extended chords, extra modifiers, top-row versus numpad physical keys, releases and repeat. Check unmodified versus modified Enter/Tab. In the app verify preceding-frame focus, focus loss/regain, active IME and an IME-event frame, Text-before-Commit duplicate suppression, Unicode IME commit, migration-modal exclusion, and key ordering (text immediately before Enter/Tab). Verify role policy, Parenthetical generation, transactional rejection, caret, dirty state and undo/redo. | **Outcome:** Not run. **Evidence:** |
| V4 | Menus, native menus and toolbar | Activate every enabled File/Edit menu item and enabled toolbar action/Role choice. Confirm stable IDs converge on one dispatcher, fresh availability is honored, one physical activation causes at most one operation, disabled placeholders remain inert, focus returns as intended, and status/error/reveal behavior is retained. On macOS cover native Application/shared menus with the in-window preview enabled and verify native accelerator ownership/no duplicate dispatch. On Windows cover in-window menu ownership and the Redo alias. Record native predefined About/Hide/Hide Others/Show All/Quit as OS-owned services, not ScriptCraft command dispatch. | **Outcome:** Not run. **Evidence:** |
| V5 | Selection, pointer and navigation | Check pointer press/click/drag/release, title-page exclusion, nearest body page, focus request ordering, selection rendering and invalid-position rejection. Check Left/Right collapse/extend, Unicode scalar boundaries, wrapped-line Up/Down, short-line clamping, Shift anchor, retained/reset preferred vertical goal, Home/End and document/page boundaries. Confirm Selection commits use the semantic executor and scrolling/reveal behavior remains unchanged. | **Outcome:** Not run. **Evidence:** |
| V6 | Document lifecycle and modal | Exercise New/Open/Close/Save/Save As/PDF; Open and Save As picker cancel/success and direct-path commands; non-UTF-8 picker-path rejection; invalid extensions and missing/malformed/duplicate/non-current FDX versions; migration Continue/Cancel, dirty-session refusal and one-shot activation; restored TextState Selection including invalid fallback; TextState zoom; failed-save rollback/conflict and Save-versus-Save-As mode; fixed `scriptcraft.pdf` export path; save/reopen source preservation. Confirm the active document and destination remain unchanged on failures. | **Outcome:** Not run. **Evidence:** |
| V7 | Clipboard and effect failures | Exercise Copy/Cut/Paste with collapsed, noncollapsed, Unicode and cross-element selections; empty, non-text and unavailable clipboard; failed Cut write; failed Paste read; verify Cut never deletes before successful write, Paste only inserts nonempty readable text, and existing status/error remains authoritative. Exercise viewport Close, imported one-shot restore, fullscreen requests, native notification pending/accepted/unavailable results and confirm acceptance is not delivery confirmation. | **Outcome:** Not run. **Evidence:** |
| V8 | View, status, appearance and timer | Exercise zoom slider/presets, native pinch, anchor behavior, pinch baseline and manual-scroll precedence; verify saved TextState zoom after save/reopen. Check System/Day/Night/Midnight appearance. Check Focus Mode availability gates, pending state, timeout/recovery and external fullscreen changes. Check timer open/close/button/outside/Escape, Countdown/Stopwatch gates, numeric filtering and 1–1440 range, Enter-to-start, Start/Pause/Resume/Reset/End, completion, notification feedback, popup cleanup, repaint scheduling, and existing status/error wording. | **Outcome:** Not run. **Evidence:** |
| V9 | Preservation and unchanged editing policy | Across ordinary editing, Role changes, navigation, pointer selection, undo/redo and save/reopen, verify exact canonical UTF-8 text/case, source preservation, semantic Role, history, dirty state, revision and failure rollback. Confirm all AC-WRITING-001.4a Enter/Tab matrices, blank-only Character-to-Transition, Dialogue split behavior, strict boundaries, and source-safe failures are unchanged. | **Outcome:** Not run. **Evidence:** |
| V10 | User-run repository checks and application | If appropriate to the user's environment and project policy, run the recorded workspace format, check, build, Clippy and test commands listed in [project-manifest.md](../../../.forge/project-manifest.md). The recorded command inventory is `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`; the historical outcomes in the manifest are not fresh results. Run/inspect the app using `cargo run -p scriptcraft` if desired, then perform native macOS and Windows interaction where available. Record exact command, exit status, output/artifact and target. | **Outcome:** Not run. **Evidence:** |

## Acceptance and disposition

Do not mark WO-008 or the initiative accepted based on this handoff. The user decides acceptance after
reviewing the complete evidence, including unresolved platform coverage. Record each defect, blocker,
scope decision, retest and final disposition here or in a linked user-owned evidence record. The
implementation submission records one corrected source bypass (Open-restored Selection now enters the
headless executor); it does not assert that the correction compiles or behaves correctly.

| Final user disposition | Evidence link / notes |
|---|---|
| Not recorded | |

# Work Order: WO-001 — Seamless Native Titlebar

## Metadata

- **Status:** `blocked` — fresh validation recorded a failed native double-click preservation criterion; the six-item visual gate is not itemized as passed. WO-001 is not complete.
- **Assignee:** One fresh desktop implementer, only after fresh-context design review passes and separate human design approval is recorded.
- **Phase/order:** First of two sequential work orders; WO-002 is its authorized successor.
- **Initiative:** [`../seamless-native-titlebar-v1.md`](../seamless-native-titlebar-v1.md)
- **Design review:** [`../reviews/design-review.md`](../reviews/design-review.md) — fresh alignment review `PASS`, no blockers or warnings.
- **Generated filename:** `WO-001-seamless-native-titlebar.md`
- **Owner/orchestrator:** Forge orchestrator.
- **Validation:** [`../validations/validation-WO-001.md`](../validations/validation-WO-001.md:1) — fresh validation recorded `fail`; WO-001 required next state is `blocked`.
- **Fixes:** `none`; this order explicitly forbids serial speculative configuration/fix attempts.
- **Implementer session/agent ID:** `fresh code implementer session; unique session/agent ID not exposed by the execution interface`
- **Session exception:** `none`

> **Revision note — 2026-10-28:** Toolbar clearance no longer depends on a titlebar-safe-area/native-button-bounds API. The implementer may choose one conservative, bounded app-layout inset/position as part of the single native configuration attempt; deterministic geometry tests and the mandatory restored/maximized user visual gate decide whether it is acceptable. This preserves native controls and ownership while pursuing the initiative's seamless surface goal.

## Summary

On the existing ordinary native macOS window path, make one evidence-based attempt to hide native `ScriptCraft` title text and make the native titlebar/content surface visually continuous, while retaining usable native traffic lights, a toolbar safely clear of native controls, and V4-WO-001's observed no-swell/no-shrink/no-snap result for both maximize and restore. The V4 observation is a preservation baseline, not a promise that option values cause the visual result. If the exact resolved API cannot express the requested native configuration while retaining decorations and native controls, or the one attempt fails any visual criterion, stop with exact blocked/failed evidence; absence of a native bounds API alone is not a blocker. Do not add another attempt or custom behavior.

## Dependencies and evidence

- **Predecessor evidence:** [`validation-V4-WO-001.md`](../../2026-10-27-titlebar-transition-and-elements-alignment/validations/validation-V4-WO-001.md) records V4-WO-001 complete/validated, visible ordinary native chrome and usable toolbar, and no swelling, shrinking, or final snap in separate maximize and restore observations.
- **Unresolved user-observed appearance:** the user's V4 report says native `ScriptCraft` title text appears beside the traffic lights and the titlebar color remains visibly distinct. No causal diagnosis is established.
- **Preservation evidence:** [`validation-V3-WO-001b.md`](../../2026-10-27-titlebar-transition-and-elements-alignment/validations/validation-V3-WO-001b.md) records the accepted black-bar correction, matching clear color, restored 2-point toolbar bottom padding, and user visual pass. It is preservation evidence only; do not reopen its scope.
- **Terminated lineage:** the v3 transition-policy/native-diagnostic chain is terminated historical context. Do not reopen, extend, or repair it. V4-WO-002 is a draft proposal and evidence input only, not an approved recipe or executable order.
- **Current startup/API evidence:** current app startup selects `NativeMacOs`, uses `eframe::NativeOptions::default()` and the `ScriptCraft` runner title, omits the app-owned title strip and app maximize/policy path in native mode, and keeps the toolbar in app content. Current lockfile evidence identifies eframe/egui-winit `0.29.1` and winit `0.30.13`; the implementer must verify exact resolved packages and reread relevant source before editing.

## In scope

- Before changes, audit the exact locked eframe, egui-winit, and winit source/API, macOS mapping and decoration semantics, effective title/titlebar/button settings, option precedence/order, complete startup handoff, and app-surface fill/clear-color relationship. Record exact versions, symbols/source locations, effective mapping, and limitations.
- Resolve conflicting historical statements about `titlebar_buttons_shown` from the actually resolved source, including boolean polarity and whether decoration changes overwrite titlebar properties. V4-WO-002's candidate API list is a lead only.
- The locked eframe viewport mechanism supports the single candidate request tuple for a decorated opaque window, hidden title text, transparent native titlebar/full-size content, and explicitly shown native buttons: `with_decorations(true)`, `with_transparent(false)`, `with_title_shown(false)`, `with_titlebar_shown(false)`, `with_fullsize_content_view(true)`, and `with_titlebar_buttons_shown(true)`. Reverify each exact mapping and ordering in current source before implementation; this is a request, not a preapproved recipe or evidence of appearance.
- If and only if that audit identifies one supported, internally consistent configuration that retains native decorations and traffic lights while requesting hidden native title text and titlebar/content blending, make exactly one minimal configuration attempt on the existing native startup path. Keep the overall window opaque unless audited API behavior requires otherwise; do not confuse whole-window transparency with native titlebar transparency.
- Retain native OS ownership of traffic lights, native maximize/restore and native double-click behavior. The app must not add custom titlebar double-click handling, app-owned maximize/restore commands, transition policy, animation workaround, or synthetic native controls.
- Keep toolbar content usable, legible, and safely clear of native traffic-light hit regions in restored and maximized states. A conservative, bounded app-layout exclusion/inset/position may place the toolbar clear of that area; no native bounds/safe-area API is a prerequisite.
- Add/adjust only focused deterministic option-selection, startup-handoff, native-ownership, and preservation tests. Tests establish requested configuration and app behavior only; they do not prove native appearance.
- Preserve the black-bar/clear-color/2-point bottom-padding result, toolbar allocation/overflow and `128.0`-point Elements width, Elements role/popup behavior, cursor ownership, input/IME, accessibility/semantic behavior, and page content.
- Run required clean Cargo gates and obtain the mandatory direct user visual gate on the actual clean-built macOS app.
- Apply only the authorized conditional requirements/blueprint edits specified under Durable-document impact, and only if implementation is approved and the complete visual gate passes.

## Out of scope

- Reopening or changing any closed v3/v4 history, the terminated v3 transition-policy/native-diagnostic chain, V4-WO-001 evidence, or the V4-WO-002 proposal.
- A second or serial speculative native configuration attempt, open-ended diagnostic loop, new options matrix or selector, unbounded layout-tuning/retry loop, unsupported native hooks, private APIs, overlays/fake chrome, whole-window borderless workaround, or loss/replacement of native traffic lights. A single conservative, bounded app-layout adjustment within the one attempt is permitted for toolbar clearance.
- App-owned maximize/restore, custom double-click behavior, custom transition policy/animation, metric hold/release, global egui zoom, polling, or changes to native maximize/restore mechanics.
- Toolbar/Elements redesign, unrelated geometry or behavior changes, dependency changes unless exact source evidence shows the locked API cannot express the required request, broad theme redesign, editing semantics, or unrelated cleanup. The current-source investigation found no relevant benefit from upgrading.
- Source/API option values as a substitute for direct visual evidence or a claim of pixel equality/cross-platform consistency.
- Any review/validation artifact during implementation, or any implementation before the separate design-review and human-approval gates.

## Requirements

Applicable text from [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md):

> **REQ-WRITING-LOOP-005: Desktop command toolbar**
>
> **User Story:** As an individual screenwriter, I want desktop commands in a consistent toolbar, so common project actions are discoverable without changing the semantic editing model.

- **AC-WRITING-005.4:** Given the current caret/focus is within a screenplay element, the Elements Item shall display that element's explicit role using the seven supported labels Scene Heading, Action, Character, Dialogue, Parenthetical, Transition, and Note; choosing a label shall apply the role through the existing transactional session operation and preserve the document on rejection.
- **AC-WRITING-005.5:** Given a toolbar width, Space shall consume one deterministic 72-point width, rendered normal Items shall consume their complete group width including label text measured at runtime with the active egui style/font, and Flexible Space Items shall divide the remaining width equally after the shared 4-point horizontal row-padding and 4-point inter-item-gap budget, and a narrow toolbar shall retain the leftmost fitting Items while exposing non-spacing Items through a far-right `>>` text menu. Action entries and Elements role choices in that menu shall remain executable without persisting toolbar layout state; a fully fitting composition shall keep its rightmost Item, including its label, inside the actual clipped toolbar content right edge after panel, frame, and viewport insets are applied. Items with an approved typed width override, including the Elements control, shall use that same resolved width for allocation and rendering without changing the shared padding, gap, flexible-space, overflow, or clipped-edge budget.
- **AC-WRITING-005.6 (current text; conditionally superseded only by an approved, visually verified update):** Given the desktop app is running, a dedicated compact title strip shall display centered bold `ScriptCraft` placeholder text above the toolbar. A double-click shall request maximize/unmaximize only when the viewport's maximized state is known; unavailable or ignored state shall use a bounded no-op/native fallback rather than an unsafe or exact-restoration claim that the platform cannot support. This work order's selected native path must not implement custom titlebar double-click handling; the existing requirement therefore requires a narrowly scoped authorized update only if the new behavior is adopted after the visual gate passes.
- **AC-WRITING-005.7:** Toolbar action controls and the Elements control/label surface shall request the standard arrow cursor, while the existing whole-document editing response shall request the text/I-beam cursor. Decorative labels and painted page rectangles shall not create competing cursor owners.

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md) — currently documents native presentation hints, a unified app-owned title-strip/toolbar surface and app-owned double-click behavior, and the toolbar/content contracts. Verify preserved contracts; conditionally reconcile only titlebar ownership/presentation and the toolbar-safe native content boundary if the adopted visual result passes.

## Implementation plan and hard stop

1. Read-only preflight: confirm branch/base/worktree per Forge dispatch policy; inspect exact locked crate versions and source for the eframe viewport builders, egui-winit mapping, winit macOS extension behavior, decorations/property ordering, title visibility, titlebar transparency/full-size content, and native-button visibility. Trace the exact options object to the sole runner handoff and confirm no later overwrite. The absence of an exposed titlebar-safe-area/native-button-bounds API is a known limitation, not by itself a reason to block before edits.
2. Record the API audit before editing. If no single supported tuple can simultaneously request hidden native title text and a blended titlebar/content surface while retaining native decorations/buttons and native controls, do not edit source. Mark the work blocked with exact resolved versions, source/API symbols and locations, mapping/polarity, and the reason the requested native configuration is unsupported. Do not block solely because no native bounds API exists.
3. If the audit supports it, make exactly one minimal, evidence-based native startup/content configuration attempt. As part of that same attempt, choose and implement one conservative, bounded app-layout exclusion/inset/position that keeps the toolbar outside the expected native traffic-light area while retaining the full-width backing surface. The exact numeric adjustment is an implementation decision, not a prescribed value or claim of native bounds; validate actual clearance in the mandatory restored/maximized macOS visual gate. Ordinary layout tuning to select that one placement is distinct from changing/retrying the native configuration. No alternate mode, second tuple, fallback override, custom hit target, or unbounded layout-tuning/retry loop is allowed. If the placement cannot be implemented within these constraints or the visual gate finds inadequate clearance, stop and report; do not hide/remove native controls, add private/unsafe hooks, or change native maximize/restore ownership.
4. Add focused deterministic tests for the exact chosen options and mode selection/handoff, native titlebar ownership without app maximize/double-click/transition handling, and preserved toolbar/content invariants, including the chosen bounded toolbar-placement geometry against content/clip bounds. Assert the correct effective API intent and app geometry, not visual outcomes. Keep existing behavior tests for toolbar allocation, 128-point Elements, popup, cursor, input/IME, and semantics passing.
5. Run the clean build and required commands. Then stop for the user's actual-window visual gate. Record the clean build/run identity and each visual criterion separately. A failed visual criterion is blocked/failed acceptance, not partial success, and authorizes no further speculative configuration attempt.

## Files and systems

- **Create:** none.
- **Update after review and separate human design approval:** `apps/scriptcraft/src/main.rs` and focused colocated tests only, plus this work order and `execution-log.md` during authorized execution. Update the relevant requirement and blueprint narrowly only if the full user visual gate passes and the behavior is adopted.
- **Inspect read-only:** `Cargo.toml`, `apps/scriptcraft/Cargo.toml`, `Cargo.lock`, exact resolved eframe/egui-winit/winit source/API, and relevant Forge predecessor evidence.
- **Avoid changing:** core source/tests, dependency manifests/lockfile unless exact source evidence establishes that the locked API cannot express the required request, unrelated app source/tests, all v3/v4 and closed historical artifacts, V4-WO-002, unrelated requirements/blueprints, review/validation artifacts, Git/branch state during this planning task, and unrelated transition-policy code. The current-source investigation found no relevant benefit from upgrading.

## Validation and E2E acceptance tests

- **Focused deterministic tests:** verify selected native options and exact single startup handoff; supported effective mapping of title-hidden/titlebar/content/button settings; native-mode exclusion of app-owned title strip, maximize command, custom double-click response, and transition policy; deterministic geometry of the chosen bounded toolbar placement against app content/clip bounds; black-bar/clear-color/padding and toolbar/Elements/cursor/popup/input/IME/semantic preservation. Tests must not claim OS appearance or native controls from option values alone.
- **Required clean Cargo gates:** `cargo clean`; `cargo fmt --all -- --check`; focused tests; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; and `git diff --check`. Record exact results and failures. After the clean build, launch the app with `cargo run -p scriptcraft` for the required user visual inspection and record the run/build identity.
- **Mandatory user visual gate — actual clean-built macOS app:** inspect restored and maximized states and perform maximize and restore separately, watching each complete native animation. The user must confirm all of the following:
  1. No native `ScriptCraft` title text is visible beside the traffic lights.
  2. Native titlebar and app content visually read as one continuous surface, with no visibly distinct titlebar color band or seam.
  3. Native traffic lights are visible and usable.
  4. Toolbar is legible and usable, with no overlap or occlusion and safe clearance from native controls in both restored and maximized states; the chosen bounded app-layout placement is acceptable in both states.
  5. Maximize completes without swelling, shrinking, or final snap.
  6. Restore completes without swelling, shrinking, or final snap.
- **Acceptance test:** Given the approved single native configuration attempt, its one bounded toolbar-layout placement, and a clean-built app on macOS, when the user inspects both window states and separately maximizes/restores, then all six observations above pass and deterministic preservation/geometry tests plus all required Cargo gates pass. Otherwise the order is blocked/failed and no second native configuration attempt or post-gate retry is made.
- **Platform boundary:** Native title suppression, native surface blending, and traffic-light presentation are platform-specific. Direct user visual evidence is the only acceptance oracle for these appearances; source, test, or option values cannot substitute.
- **Fresh-session boundary:** after implementation and complete evidence, dispatch a separate fresh-context validator. Do not create the validation artifact now, do not self-validate, and do not mark this order complete before validator disposition.

## Durable-document impact

- **Requirements impact:** `update` — affected document/ID: [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md), `AC-WRITING-005.6`. Rationale: current text promises a dedicated app-owned title strip and app-owned known-state double-click maximize behavior, directly conflicting with the requested titleless native surface and native-owned maximize/restore path. Authorized change is conditional on approved design, approved implementation, and passing full visual gate; otherwise leave unchanged and record why.
- **Blueprint impact:** `update` — affected document: [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md), native presentation and unified chrome ownership contracts. Rationale: the blueprint currently describes app-owned title-strip/double-click behavior and native presentation hints, while the adopted design would establish a platform-specific native titlebar/content boundary and safe toolbar placement. Authorized change is conditional on approved design, implementation evidence, and passing full visual gate; otherwise leave unchanged and record why.
- **Authorized documentation changes:** only the narrow AC-WRITING-005.6 titlebar ownership/interaction description and corresponding desktop blueprint native presentation/chrome boundary, after all gates pass. No edits are authorized during this planning task.

## Completion checklist

- [x] Fresh implementer performed the exact resolved API audit and recorded one supported configuration attempt or the exact blocked reason.
- [x] Focused deterministic tests and all required Cargo gates have recorded evidence.
- [ ] User passed every visual criterion in restored/maximized states and both complete animation directions.
- [x] Conditional requirement/blueprint updates were applied only if authorized and supported by passed visual evidence; otherwise unchanged status and rationale are recorded. The visual gate is pending, so both durable documents remain unchanged.
- [x] Execution log records implementation evidence, changed files, comparable diff, exact user observation, and the `execute → validate` transition. WO-001 is `in-review`; its full visual gate remains pending and validation is not yet complete.
- [ ] Separate fresh-context validation passed; only that validator disposition may mark the work order `complete`.

## Implementation evidence — 2026-10-28

### Pre-edit branch and API audit

- **Branch/worktree:** Current branch is `dev/2026-10-27-seamless-native-titlebar`; `HEAD` and `master` are both `65b1de6bc2ee2b87625a9ce543769e1920ee45fc`. There were no staged or modified tracked files before source edits. The initiative directory was untracked and contained the plan, work order, reviews, log, and placeholder artifacts created during planning/review; this is expected expansion from the earlier branch-creation inventory, not an unexplained source/worktree change. No branch discrepancy was found.
- **Resolved packages:** `Cargo.lock` resolves eframe `0.29.1`, egui-winit `0.29.1`, and winit `0.30.13`. The exact cached sources were present and inspected read-only. The failed first lookup using `rg` (`rg` is unavailable in this environment) was immediately replaced by `grep`/`sed` source inspection; no dependency change was warranted.
- **Resolved API semantics:** `egui-0.29.1/src/viewport.rs` documents `with_decorations(true)` and `with_transparent(false)` for decorated/opaque intent; `with_title_shown(false)` hides title text; `with_titlebar_shown(false)` makes the titlebar transparent (not hidden); `with_fullsize_content_view(true)` extends content behind it; and `with_titlebar_buttons_shown(true)` explicitly requests visible buttons. `egui-winit-0.29.1/src/lib.rs:1610-1616` builds base attributes, including decorations, first; its macOS mapping at `:1732-1738` applies the title-hidden, button-hidden, transparent-titlebar, and full-size settings afterward with inverted booleans for the first three. The winit `0.30.13` macOS docs note that decoration changes can overwrite titlebar properties, so this ordering is consequential. `window_delegate.rs:535-550` selects the normal titled/resizable/closable/minimizable style when decorations are enabled and titlebar-hidden is false; `:590-615` applies titlebar transparency and hidden-title state and hides standard buttons only when the hidden-buttons flag is true. The distinct `with_titlebar_hidden(true)` borderless path is not used.
- **Startup/effective path:** `TITLEBAR_PRESENTATION_MODE` selects `NativeMacOs`; `native_options_for_mode()` supplies the single native tuple below; `startup_presentation()` carries that exact options object to the sole `eframe::run_native("ScriptCraft", options, ...)` call. No later app-side option rewrite was found. The runner title identity remains `ScriptCraft`; the selected mode omits the app-owned title strip and does not send app-owned maximize/restore commands or apply the app-owned transition policy.
- **Surface evidence/limits:** `toolbar_chrome_color()` and `chrome_clear_color()` both derive from `visuals.widgets.open.weak_bg_fill`, and the top-panel/central-panel frames use that fill. This preserves the app surface/clear-color contract but does not prove macOS compositor blending. The audited public stack exposes no titlebar safe-area or traffic-light bounds to app layout; the WO explicitly allows the bounded layout choice, so absence of such an API was not treated as a blocker.

### Implemented attempt and preservation

- The selected native mode now requests exactly: decorations `true`; whole-window transparency `false`; title shown `false`; titlebar shown `false` (transparent); full-size content view `true`; titlebar buttons shown `true`. The native configuration is applied once; the locked dependency manifests and `Cargo.lock` are unchanged.
- Added one local `28.0`-point native-only top inset before the toolbar. The top panel's full-width backing fill remains intact, toolbar width/allocation code and width constants remain unchanged, and the inset is zero in the existing non-native mode. The inset is a conservative bounded layout choice—not a reported native safe area or proof of actual control clearance. Direct restored/maximized inspection decides clearance.
- Added/updated deterministic tests for the exact selected viewport tuple and startup handoff, and for the native-only 28-point reserved band preserving full toolbar width/height geometry. Existing toolbar, Elements width, allocation, popup, cursor, input/IME, semantic, page-content, clear-color, and native ownership tests were not removed or replaced.
- **Source diff:** `git diff -- apps/scriptcraft/src/main.rs` is the comparable source diff. Exact implementation changes are limited to that source file; no core files, dependency files, requirements, or blueprints changed.

### Commands and results

All required commands were run after `cargo clean` on the current macOS `26.6.2` / `aarch64-apple-darwin` host with Rust/Cargo `1.99.0`:

| Command | Result |
|---|---|
| `cargo clean` | Passed; removed 2,977 files / 875.6 MiB before gates. |
| `cargo fmt --all -- --check` | Passed (exit 0). |
| `cargo test -p scriptcraft native_` | Passed: 2 focused tests, 0 failed, 87 filtered. |
| `cargo check --workspace` | Passed (exit 0). |
| `cargo test --workspace` | Passed: app 89; core unit tests 45; integration groups 2, 5, 4, 6, 2, 10, 5, and 3; doc tests 0. Total 171 passed, 0 failed. |
| `cargo build --workspace` | Passed (exit 0). |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed (exit 0). |
| `git diff --check` | Passed (exit 0). |
| `cargo run -p scriptcraft` | Launched the clean-built `target/debug/scriptcraft`; process remains active for direct user macOS inspection. This launch is not visual acceptance evidence. |

- **Source-audit command failure:** the initial shell lookup pipeline used `rg -n ...`; it exited 127 with `/bin/sh: rg: command not found`. Re-running the inspection with `grep -n` and `sed -n` succeeded. This tooling failure did not affect source or tests.
- **Build/run identity:** clean workspace build on macOS `26.6.2`, host `aarch64-apple-darwin`, Rust `1.99.0 (b940084d7 2026-09-28)`, Cargo `1.99.0 (5f94df478 2026-08-27)`. The launched executable is `target/debug/scriptcraft` from that workspace build.

### User visual gate and disposition

- **Status:** `pending` — the implementer did not perform or claim the human visual inspection. The launched native app is available for the user's direct inspection.
- **User observation, recorded verbatim:** “double click functionality does not work, everything else looks good”. This is the only new user visual evidence recorded here. It does not itemize the WO's visual criteria or establish that each criterion passed; the complete visual gate remains unconfirmed.
- Still required separately: confirm hidden native title text; continuous titlebar/content with no distinct band/seam; visible and usable native traffic lights; legible/usable toolbar with safe clearance in restored and maximized states; and complete native maximize and restore animations without swelling, shrinking, or final snap.
- Requirements and blueprint remain unchanged because the complete visual gate has not passed; the conditional narrow reconciliation is not yet authorized. No validator was dispatched and no validation artifact was created.
- **Current disposition:** WO-001 is `in-review` for the separate fresh validation loop, with only the implementation/test evidence already recorded and the exact user observation above. This status does not claim full visual acceptance or completion. The validator must record the incomplete visual evidence as such.

## Planning and approval boundary

- The initial review failure remains historical; the fresh alignment review of this revised WO passed with no blockers or warnings. The user explicitly directed that a separate implementer be deployed after an aligned review; implementation and automated evidence are recorded above. The user observation is recorded verbatim, but no itemized visual pass is claimed. WO-001 is now `in-review` with the available evidence; separate fresh validation has not yet been dispatched.
- The alignment review and execution authorization are recorded in the initiative plan and [`design-review.md`](../reviews/design-review.md); implementation remains limited to the scope and gates in this order.
- Any API-level or visual failure stops this one-order attempt; no serial fix chain or speculative diagnostics are authorized by this work order.

## Fresh validation disposition — 2026-10-28

- **Outcome/status:** `fail`; required next state `blocked`. Do not mark WO-001 complete.
- **Known defect:** The exact user observation is “double click functionality does not work, everything else looks good”. This fails the explicit native double-click behavior preservation criterion. The observation does not establish any of the six visual criteria individually; each remains unverified rather than assumed passed.
- **Evidence:** [`validation-WO-001.md`](../validations/validation-WO-001.md:1) records criteria-specific results, the diff/evidence comparison, and the implementer-attributed (not rerun) checks.
- **Boundary:** No application code or WO-002 artifact was changed. No duplicate executable fix order was created; WO-002 remains a separately authorized successor, not a WO-001 fix chain.

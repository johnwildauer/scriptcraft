# Work Order: V4-WO-001 — Vanilla Native Titlebar Comparison

## Metadata

- **Status:** `complete`
- **Assignee:** Fresh desktop implementer for V4-WO-001 only.
- **Phase/order:** First and only work order in the v4 experiment.
- **Initiative:** [`titlebar-transition-and-elements-alignment-v4.md`](../titlebar-transition-and-elements-alignment-v4.md:1)
- **Generated filename:** [`V4-WO-001-vanilla-native-titlebar-comparison.md`](V4-WO-001-vanilla-native-titlebar-comparison.md:1)
- **Owner/orchestrator:** Forge orchestrator.
- **Validation:** [`validation-V4-WO-001.md`](../validations/validation-V4-WO-001.md:1) — fresh validation returned `PASS WITH WARNINGS`, with no hard-stop and a recommendation of `complete`.
- **Fixes:** `none`.
- **Implementer session/agent ID:** `implementer-fresh-v4-wo001-native-titlebar-2026-10-27T2149Z`
- **Session exception:** `none`.

## Summary

Establish one ordinary, visibly native macOS titlebar startup baseline using the resolved framework/platform defaults rather than another titlebar diagnostic configuration. Keep the existing custom hidden-titlebar path available for rollback, preserve the validated toolbar/content contracts, and obtain the user's direct observation of native maximize and restore so the result can be compared with the already confirmed custom-path swell/shrink/snap artifact. This is a reversible experiment; either animation result is valid evidence and neither constitutes a permanent product decision.

## In scope

- Audit the effective defaults and startup path for the resolved eframe/egui-winit/winit versions before changing native presentation configuration.
- Make the smallest reversible startup change needed to select the ordinary decorated native macOS window path for this experiment; retain only unrelated options necessary for normal app startup and the product window identity.
- Ensure native window chrome owns the titlebar and its native maximize/restore behavior in the experiment. Do not render the custom title strip, transparent/hidden titlebar substitute, duplicate title surface, or app-owned title-strip maximize response over the native titlebar.
- Keep the existing custom hidden-titlebar path intact as a rollback/comparison path. Do not add a new user-facing mode selector or additional native diagnostic mode/options matrix.
- Keep the egui toolbar as app content and preserve toolbar allocation, the `128.0` Elements width, black-bar/clear-color/restored-padding behavior, Elements/popup/cursor/input/IME/accessibility/semantic behavior, and page content.
- Add or adjust only focused deterministic tests for startup selection, ordinary native presentation configuration, reversible custom path, and preserved ownership/contracts.
- Obtain and record one consolidated user visual checkpoint after the clean rebuilt app launches: first confirm native chrome is actually visible; only then observe double-click maximize and restore animation.

## Out of scope

- A permanent product choice between native and custom titlebars, or a claim that the native titlebar fixes the scaling artifact.
- Further iterative native-diagnostic attempts, startup diagnostic logging, a new options/feature toggle, global egui zoom, native animation controls, desktop/work-area polling, blind overlays, or speculative geometry changes.
- Removing accumulated custom transition helpers or broad cleanup/refactoring.
- Toolbar compression, black-bar correction, Elements layout/width changes, semantic editing changes, dependency changes, requirements/blueprint changes, or modifications to v1/v2/v3 artifacts.
- Creating a validation result, performing Git/branch actions, dispatching implementation/validation from this planning artifact, or doing implementation in the planning session.

## Requirements

Applicable text from [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:90):

> **REQ-WRITING-LOOP-005: Desktop command toolbar**
>
> **User Story:** As an individual screenwriter, I want desktop commands in a consistent toolbar, so common project actions are discoverable without changing the semantic editing model.

- [`AC-WRITING-005.5`](../../../requirements/features/screenplay-writing-loop.md:115): Given a toolbar width, Space shall consume one deterministic 72-point width, rendered normal Items shall consume their complete group width including label text measured at runtime with the active egui style/font, and Flexible Space Items shall divide the remaining width equally after the shared 4-point horizontal row-padding and 4-point inter-item-gap budget, and a narrow toolbar shall retain the leftmost fitting Items while exposing non-spacing Items through a far-right `>>` text menu. Action entries and Elements role choices in that menu shall remain executable without persisting toolbar layout state; a fully fitting composition shall keep its rightmost Item, including its label, inside the actual clipped toolbar content right edge after panel, frame, and viewport insets are applied. Items with an approved typed width override, including the Elements control, shall use that same resolved width for allocation and rendering without changing the shared padding, gap, flexible-space, overflow, or clipped-edge budget.
- [`AC-WRITING-005.6`](../../../requirements/features/screenplay-writing-loop.md:131): Given the desktop app is running, a dedicated compact title strip shall display centered bold `ScriptCraft` placeholder text above the toolbar. A double-click shall request maximize/unmaximize only when the viewport's maximized state is known; unavailable or ignored state shall use a bounded no-op/native fallback rather than an unsafe or exact-restoration claim that the platform cannot support.
- [`AC-WRITING-005.7`](../../../requirements/features/screenplay-writing-loop.md:137): Toolbar action controls and the Elements control/label surface shall request the standard arrow cursor, while the existing whole-document editing response shall request the text/I-beam cursor. Decorative labels and painted page rectangles shall not create competing cursor owners.

These remain the durable product contract. The temporary experiment bypasses the custom title strip only in the selected test startup path; it must retain the custom path for rollback and must not reinterpret the experiment as a requirement change.

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:105) — describes current native presentation hints and the unified app-owned titlebar/toolbar surface. Treat it as the durable product contract during this temporary experiment; native chrome owns the titlebar only in the experiment's selected startup path.

## Implementation plan

1. Inspect the resolved eframe, egui-winit, and winit versions and trace the complete selected startup path, including defaults for decorations, title, transparency, title visibility, titlebar visibility, and full-size content. Record which existing explicit settings are genuinely needed for app startup and which make the current native diagnostic non-vanilla. Do not infer that every explicit option should be deleted.
2. In the app startup code, select the ordinary native decorated window path as the experiment baseline, with the normal product window identity and no diagnostic title or titlebar-specific override layer. Keep unrelated startup sizing/options intact when the audit shows they are needed. Remove the custom title-strip rendering and app-owned title-strip maximize response from this selected path only; retain the existing custom hidden-titlebar path and transition behavior unchanged for rollback.
3. Add/adjust focused tests that establish the native baseline is the selected startup path, native decoration is not explicitly hidden/transparent/full-size-overlay configured, the custom rollback path remains reachable through the existing source-level mechanism, the selected startup handoff has no later override, and preserved toolbar/content contracts remain intact. Do not expand into general presentation refactoring.
4. Run the required clean build and automated checks. Then provide the user the single ordered visual checkpoint described below. Record the clean build identity and the user's two animation observations in the work order and execution log; do not create or edit a validation artifact.

## Files and systems

- **Create:** none.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1) and focused tests colocated with the startup/presentation code; [`execution-log.md`](../execution-log.md:1) only during the later authorized execution phase; this work order only to record implementation evidence and gate results.
- **Inspect read-only:** [`apps/scriptcraft/Cargo.toml`](../../../../apps/scriptcraft/Cargo.toml:1), [`Cargo.lock`](../../../../Cargo.lock:1), and the installed resolved eframe/egui-winit/winit API sources.
- **Avoid changing:** all other application/core source and tests, dependency manifests/lockfile, requirements, blueprints, v1/v2/v3 artifacts, review and validation artifacts, toolbar geometry, black-bar ownership/padding/clear color, Elements behavior, and unrelated transition code.

## Validation and E2E acceptance tests

- **Automated checks:** focused startup/default/rollback/preservation tests; [`cargo clean`](../../../project-manifest.md:80); [`cargo fmt --all -- --check`](../../../project-manifest.md:81); [`cargo check --workspace`](../../../project-manifest.md:84); [`cargo test --workspace`](../../../project-manifest.md:83); [`cargo build --workspace`](../../../project-manifest.md:85); [`cargo clippy --workspace --all-targets -- -D warnings`](../../../project-manifest.md:82); and [`git diff --check`](../execution-log.md:1). Record exact outcomes; these checks prove code/configuration only, not the visible macOS titlebar or animation.
- **Manual/external checks — one ordered user checkpoint:**
  1. After a clean rebuild and launch, the user confirms the ordinary native titlebar and native window controls are visibly present and non-transparent, the product title appears without a duplicate app-owned strip, and the toolbar remains app content at the native content boundary. If this prerequisite is false, stop and record the experiment setup as failed; do not use any observed animation as native comparison evidence.
  2. Only after confirmation, the user double-clicks the native titlebar to maximize and then restore while watching each complete animation. The user reports whether the toolbar icon/text and page presentation swell, shrink, or snap during maximize and during restore. Compare that report with the already confirmed custom hidden-titlebar result: the custom path showed swell/shrink/snap, and v3's app-owned metric policy did not remove it. Do not require a repeat test of the custom path.
- **Observation classification:** `same` if the native path reproduces the reported swell/shrink/snap artifact; `absent in both directions` only if neither maximize nor restore shows it; `different/partial` for any other or direction-specific movement. A valid result of any class completes the experiment's evidence objective; it does not pass or fail a product fix.
- **Acceptance test:** Given a clean rebuilt app using the ordinary native titlebar baseline, when the user first confirms visible native chrome and then double-clicks to maximize and restore, then the implementation evidence records the native-chrome prerequisite and the separate animation observations/classification against the known custom-path result, without claiming the artifact is fixed or choosing a permanent titlebar design.

## Durable-document impact

- **Requirements impact:** `verify unchanged` — [`REQ-WRITING-LOOP-005`](../../../requirements/features/screenplay-writing-loop.md:90), [`AC-WRITING-005.5`](../../../requirements/features/screenplay-writing-loop.md:115), [`AC-WRITING-005.6`](../../../requirements/features/screenplay-writing-loop.md:131), and [`AC-WRITING-005.7`](../../../requirements/features/screenplay-writing-loop.md:137) remain the durable product behavior. The experiment is temporary and reversible, and does not decide whether native chrome becomes the product default.
- **Blueprint impact:** `verify unchanged` — [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:105) remains the durable product contract; the selected native baseline is an experiment-specific startup override, not a persistent architecture decision.
- **Affected documents/IDs:** [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:90), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:105); no edits authorized.
- **Rationale and evidence:** the requirement's toolbar/title-strip contract and blueprint's unified app-owned surface were checked against this bounded experiment. The v4 purpose is evidence collection only, the custom hidden-titlebar path remains available, and the validated toolbar/black-bar behavior is explicitly preserved.
- **Authorized documentation changes:** implementation evidence in this work order and the execution log during authorized execution only. No requirements, blueprint, review, validation, v3, or historical artifact edits.

## Implementation evidence — 2026-10-27

- **Resolved API/default audit:** [`Cargo.lock`](../../../../Cargo.lock:875) resolves eframe/egui-winit `0.29.1` and winit `0.30.13`. Eframe's [`NativeOptions::default()`](../../../../apps/scriptcraft/src/main.rs:2576) uses the default viewport; egui's [[`ViewportBuilder::default()`](../../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/viewport.rs:264)](../../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/viewport.rs:264) leaves title and optional native presentation fields unset. The resolved [egui-winit mapping](../../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-winit-0.29.1/src/lib.rs:1604) defaults unset generic decoration/transparency to decorated and opaque, and macOS title/titlebar/buttons to shown with full-size content view disabled. Resolved winit defaults likewise leave titlebar-transparent/hidden/buttons-hidden/full-size-content flags false and window decorations enabled. Eframe's runner fills an unset viewport title from the `"ScriptCraft"` app-name argument at [`eframe::run_native()`](../../../../apps/scriptcraft/src/main.rs:2602). These are requested/default configuration findings only, not proof of actual visible macOS chrome. The resolved egui-winit source forwards [`titlebar_buttons_shown`](../../../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-winit-0.29.1/src/lib.rs:1736), contradicting the earlier v3 note that it is ignored.
- **Implementation:** Selected the existing source-level mode as [`NativeMacOs`](../../../../apps/scriptcraft/src/main.rs:49). It now returns framework defaults without explicit title, decoration, transparency, title visibility, titlebar/button visibility, or full-size-content overrides. The ordinary product identity remains the existing runner title. Removed the diagnostic-specific title and startup option logging. Existing mode-gated rendering/interaction omit the custom title strip and app-owned maximize response in native mode; the [`CustomHidden`](../../../../apps/scriptcraft/src/main.rs:53) path retains its explicit hidden-titlebar configuration and existing custom strip/transition behavior for rollback. No later options overwrite occurs between selected [`StartupPresentation`](../../../../apps/scriptcraft/src/main.rs:2580) and the sole runner handoff.
- **Focused tests and preservation:** Startup selection/default configuration/reversibility, custom/native surface ownership, transition-policy separation, and native/custom maximize-command tests pass. The three focused test invocations each passed 1/1. Existing V3 preservation evidence [`validation-V3-WO-001b.md`](../validations/validation-V3-WO-001b.md:48) confirms the black-bar/clear-color/restored-padding visual pass; no toolbar/content layout or ownership code was changed.
- **Clean checks:** An initial [[`cargo fmt --all -- --check`](../../../project-manifest.md:81)](../../../project-manifest.md:81) found formatter drift in new assertions; [[`cargo fmt --all`](../../../project-manifest.md:81)](../../../project-manifest.md:81) corrected it. Then [[`cargo clean`](../../../project-manifest.md:80)](../../../project-manifest.md:80) removed 4,668 generated files (1,014.2 MiB); the clean [[`cargo fmt --all -- --check`](../../../project-manifest.md:81)](../../../project-manifest.md:81), [[`cargo check --workspace`](../../../project-manifest.md:84)](../../../project-manifest.md:84), [[`cargo test --workspace`](../../../project-manifest.md:83)](../../../project-manifest.md:83), [[`cargo build --workspace`](../../../project-manifest.md:85)](../../../project-manifest.md:85), [[`cargo clippy --workspace --all-targets -- -D warnings`](../../../project-manifest.md:82)](../../../project-manifest.md:82), and [[`git diff --check`](../execution-log.md:1)](../execution-log.md:1) chain exited 0. Workspace tests passed with 88 app unit tests, 45 core unit tests, integration tests, and doc-tests. Final diff check is repeated after this evidence update.
- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:47), this work order, and [`execution-log.md`](../execution-log.md:1). The worktree was already dirty: preflight recorded modified source/log and untracked v3/v4 Forge artifacts. Those changes were preserved. No reset, checkout, Git branch action, dependency change, requirement/blueprint edit, v3/historical edit, review edit, or validation-artifact edit was made. [[`cargo clean`](../../../project-manifest.md:80)](../../../project-manifest.md:80) was the required generated-build cleanup only.
- **Visual gate/evidence:** The user confirmed native chrome is visible and the toolbar is usable, satisfying the native-chrome visibility prerequisite for the comparison. Exact user report: “Native chrome is visible and the toolbar is usable; neither maximize nor restore showed swelling, shrinking, or a final snap. this is exactly the behavior we want when maximizing the window. now the problem is that the title is back by the traffic lights (we had removed) and the bar color is still showing, preventing. seamless visual up through the entire window. we are susccesfully back to square 1.” The animation comparison is answered: neither maximize nor restore showed swelling, shrinking, or a final snap, classified as **absent in both directions** relative to the known custom-path artifact. Separately, the title appearing by the traffic lights and the visible bar color remain visual-design gaps; this is not a seamless-chrome success or a claim that the overall visual problem is fixed. No technical cause is inferred. V4-WO-001 remains `in-review` pending fresh validation.

## Completion checklist

- [x] Implementation is complete within scope and the custom hidden-titlebar rollback path remains intact.
- [x] Automated checks, the native-chrome prerequisite, and both maximize/restore user observations are recorded.
- [x] Durable-document impact classification is accurate against the diff; no unauthorized durable-document edits were made.
- [x] Execution log is appended with dispatch/completion identifiers and evidence during the authorized execution phase.
- [x] Fresh validation returned `PASS WITH WARNINGS` with no hard-stop and recommended `complete`; the work order is complete.

## Fresh-session and gate requirements

- Keep the initiative in `plan` until a separate fresh-context design review is prepared and passes. A separate human design approval is then required before execution.
- Before implementation dispatch, recheck current base and local branch capability, create/record the required initiative branch under [`project-manifest.md`](../../../project-manifest.md:67), and satisfy branch policy. No such check or Git action is part of this work order's planning.
- Dispatch one fresh implementer session for this work order and a separate fresh validator session after implementation evidence is recorded. The native visual report is a required user evidence gate but does not replace validation.
- Do not create [`validation-V4-WO-001.md`](../validations/validation-V4-WO-001.md:1) during planning. Do not mark this order approved, in progress, in review, or complete before its applicable gates and evidence exist.

# Work Order: V2-WO-004 — Integrated Native Validation and Durable-Document Reconciliation

## Metadata

- **Status:** `complete`
- **Assignee:** Cross-surface/native validation implementer (fresh session)
- **Phase/order:** 4 of 4; depends on V2-WO-001, V2-WO-002, and V2-WO-003
- **Initiative:** [`../titlebar-transition-and-elements-alignment-v2.md`](../titlebar-transition-and-elements-alignment-v2.md:1)
- **Generated filename:** `V2-WO-004-integrated-native-validation-and-document-reconciliation.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-V2-WO-004.md`](../validations/validation-V2-WO-004.md:1)
- **Fixes:** none
- **Implementer session/agent ID:** `implementer-fresh-v2-wo004-integrated-native-validation-2026-10-27T1922Z`
- **Session exception:** none

## Summary / one outcome

Run real bounded integration evidence for transition metrics, Elements cursor ownership, and toolbar-bottom surface ownership, then reconcile requirements and blueprints against the resulting source/API/native evidence. This is an implementation-capable correction boundary, not an administrative shortcut: implement only an evidenced in-scope correction, otherwise preserve source and record the limitation.

## In scope

- Compare the three preceding WO ledgers, diffs, tests, API findings, and native-smoke records against the integrated desktop-shell outcome.
- Run deterministic full-workspace checks and a usable-display macOS protocol covering maximize/restore request/state/rectangles, transition metrics, pointer cursor regions, popup/keyboard behavior, toolbar pixel ownership, and 128-point width.
- Apply the smallest evidenced source correction if an authorized mismatch remains, with a regression test and rerun of affected checks.
- Reconcile REQ-WRITING-LOOP-005 / AC-WRITING-005.4–005.7 and desktop/feature/technical blueprints; update only when the supported contract changes.

## Out of scope

- Administrative-only completion claims, design-review or human-approval substitution, retroactive validation, full-screen, filename plumbing, universal exact frame history, unsupported animation control, speculative desktop-dimension polling, global page-only egui scale locks, wholesale allocator replacement, and unrelated product behavior.

## Requirements

- **REQ-WRITING-LOOP-005:** “As an individual screenwriter, I want desktop commands in a consistent toolbar, so common project actions are discoverable without changing the semantic editing model.”
- **AC-WRITING-005.4:** “Given the current caret/focus is within a screenplay element, the Elements Item shall display that element's explicit role using the seven supported labels Scene Heading, Action, Character, Dialogue, Parenthetical, Transition, and Note; choosing a label shall apply the role through the existing transactional session operation and preserve the document on rejection.”
- **AC-WRITING-005.5:** “Given a toolbar width, Space shall consume one deterministic 72-point width, rendered normal Items shall consume their complete group width including label text measured at runtime with the active egui style/font, and Flexible Space Items shall divide the remaining width equally after the shared 4-point horizontal row-padding and 4-point inter-item-gap budget, and a narrow toolbar shall retain the leftmost fitting Items while exposing non-spacing Items through a far-right `>>` text menu. Action entries and Elements role choices in that menu shall remain executable without persisting toolbar layout state; a fully fitting composition shall keep its rightmost Item, including its label, inside the actual clipped toolbar content right edge after panel, frame, and viewport insets are applied. Items with an approved typed width override, including the Elements control, shall use that same resolved width for allocation and rendering without changing the shared padding, gap, flexible-space, overflow, or clipped-edge budget.”
- **AC-WRITING-005.6:** “Given the desktop app is running, a dedicated compact title strip shall display centered bold `ScriptCraft` placeholder text above the toolbar. A double-click shall request maximize/unmaximize only when the viewport's maximized state is known; unavailable or ignored state shall use a bounded no-op/native fallback rather than an unsafe or exact-restoration claim that the platform cannot support.”
- **AC-WRITING-005.7:** “Toolbar action controls and the Elements control/label surface shall request the standard arrow cursor, while the existing whole-document editing response shall request the text/I-beam cursor. Decorative labels and painted page rectangles shall not create competing cursor owners.”

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:59) — integrated shell, toolbar, Elements, cursor, and viewport ownership.
- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:105) — native presentation limitations and bounded request contract.
- [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:78) — desktop presentation over explicit roles/layout.
- [`technical-context.md`](../../../blueprints/technical-context.md:31) — presentation-only viewport scaling.

## Source/API findings

- Integrated source points are [`title_strip()`](../../../../apps/scriptcraft/src/main.rs:1806), [`update()`](../../../../apps/scriptcraft/src/main.rs:1831), [`elements_toolbar_item()`](../../../../apps/scriptcraft/src/main.rs:714), [`live_document_surface()`](../../../../apps/scriptcraft/src/main.rs:1126), [`presentation_metrics()`](../../../../apps/scriptcraft/src/main.rs:1579), [`chrome_frame()`](../../../../apps/scriptcraft/src/main.rs:1741), and [`native_options()`](../../../../apps/scriptcraft/src/main.rs:1899).
- Installed egui/eframe/winit evidence bounds viewport state to optional observations and `Maximized` requests; no supported timeline, requested-resize flag, or native pixel owner API exists.
- The current source baseline is `TOOLBAR_ELEMENTS_WIDTH = 128.0`; this WO is not permission to undo it.

## Implementation guidance

1. Require completed, independently validated V2-WO-001–003 evidence before dispatch.
2. Build a traceability matrix from each result to the requirements/blueprints and compare the actual diff; do not treat missing native evidence as success.
3. Run native smoke when a usable display exists; if not, record exact unavailable observations and retain bounded limitations.
4. If and only if a real in-scope defect is evidenced, implement the smallest correction and regression test; otherwise make no runtime change.
5. Apply only evidence-backed durable updates and leave design review, human approval, implementation, and validation gates pending for the initiative authoring state.

## Files and systems

- **Create:** no source by default; regression test only with a real correction; no design-review artifact.
- **Update:** source/requirements/blueprints only when evidence authorizes it; this work order and later execution evidence.
- **Avoid changing:** v1 artifacts, unrelated durable documents, filename/full-screen behavior, unsupported native contracts, or prior validation files.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; focused viewport/metric/chrome/Elements/cursor/shared-toolbar tests; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`.
- **Deterministic acceptance:** prior contracts remain green; known/unknown viewport requests are safe; any metric freeze is page-only and evidenced; Elements is standard-arrow/128-point/closed-centered/open-left; document alone owns I-beam; toolbar bottom owner is identified without blind native painting.
- **Native-smoke protocol:** on usable macOS, record OS/backend/window config/display scale; command/state/outer/inner/content/clip rectangles across both transitions; metric and raster bounds; cursor at each Elements/document region; popup mouse/keyboard/role selection; toolbar physical pixels and app/native ownership. Record unavailable/ignored/stale results explicitly.
- **Acceptance test:** Given the three bounded WO results, when integrated checks run, then the evidence either supports the combined desktop-shell contract or identifies and corrects a real in-scope mismatch; no missing native evidence is upgraded to a pass or unsupported promise.

## Dependencies, risks, and handoff

- **Dependencies:** V2-WO-001, V2-WO-002, and V2-WO-003 must be implemented and independently validated first; this WO also receives the separate fresh validator required by Forge.
- **Risks:** native display remains unavailable; OS-owned animation prevents universal metric claims; stale state or compositor ownership limits conclusions; reconciliation can conceal defects if diff comparison is skipped.
- **Handoff:** leave an auditable integrated evidence package for later design review and human approval; do not create those artifacts here.

## Durable-document impact

- **Requirements impact:** `verify unchanged` by default; `update` only if the integrated source/API/native result changes AC-WRITING-005.4–005.7.
- **Blueprint impact:** `verify unchanged` by default; `update` only if the integrated desktop-shell contract differs from the cited blueprints.
- **Affected documents/IDs:** REQ-WRITING-LOOP-005 / AC-WRITING-005.4–005.7; desktop container, feature, and technical blueprints.
- **Rationale and evidence:** this is reconciliation plus real correction authority, not a documentation-only shortcut; classification must match the final diff and evidence.
- **Authorized documentation changes:** minimal evidence-backed wording only; preserve no-full-screen, no-filename, no-exact-history, no-animation-control, no-polling, no-global-scale-lock, and no-wholesale-allocator constraints.

## Completion checklist

- [x] Integrated implementation/evidence is complete within scope.
- [x] Deterministic and native attempts/results are recorded.
- [x] Durable-document classification matches the final diff.
- [x] Fresh validation passed with a non-blocking native-display warning; work order is `complete`. See [`validation-V2-WO-004.md`](../validations/validation-V2-WO-004.md:1).

## Implementation evidence — 2026-10-27

- **Branch/status before changes:** `git branch --show-current` confirmed `dev/2026-10-27-titlebar-transition-and-elements-alignment-v2`; `HEAD` and `master` both resolved to `e77d6c4ee424403c5ab779d372745a51e4cafd4e`. The working tree contained the expected V2 source/log modifications and untracked V2 Forge artifacts. No reset, checkout, commit, or destructive Git action was used.
- **Preceding ledger comparison:** V2-WO-001, V2-WO-002, and V2-WO-003 are each marked `complete` with fresh `pass` validations. Their durable classifications and warnings were compared against the aggregate source/diff: transition diagnostics retain live-width metrics and known-state requests; Elements retains stock ComboBox behavior, non-sensing decorative caption, centered closed text, popup/keyboard routing, and `128.0` points; the chrome ledger proves contiguous logical ownership but leaves native pixel ownership unresolved. No cross-surface mismatch or unauthorized behavior was evidenced.
- **Integrated deterministic evidence:** Focused title-strip (2), presentation-metric (2), Elements cursor-ledger (1), cursor-intent (1), chrome-bottom ledger (1), fractional physical-row (1), shared-chrome (1), and toolbar-allocation (6) tests passed. The full workspace suite passed: 81 app tests, 45 core tests, integration tests, and doc-tests. `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` all exited 0. Existing deterministic evidence remains exact: unified panel/title/toolbar geometry has zero seam, normal/maximized-equivalent metrics remain page width `576.0`, scale `0.941176`, font `11.294117`, Elements remains `128.0`, and the chrome ledger's fractional bottom-padding span is `[165,173)` at 2 pixels/point.
- **Native availability/protocol:** macOS/Darwin arm64 was confirmed. `DISPLAY` and `WAYLAND_DISPLAY` were unset. A bounded `cargo run -p scriptcraft` launch ran for 8 seconds and remained alive with only Cargo/target launch output; it was then terminated cleanly for the bounded attempt. `System Events` was unavailable because osascript reported assistive access denied (`-25211`), and `screencapture` reported `could not create image from display`. Consequently maximize/restore clicks and command/state/outer/inner/content/clip rectangles, transition frames/metrics, Elements cursor/popup/keyboard behavior, toolbar physical pixels, display scale, and unified-chrome native ownership are unavailable and unclaimed.
- **Correction decision:** No real in-scope mismatch is evidenced by the integrated source/API/deterministic boundary, and the required native evidence is unavailable. No speculative source change, regression test, blind native paint, geometry/padding change, allocator replacement, global egui zoom lock, desktop-dimension polling, animation control, full-screen behavior, filename plumbing, or exact restore-frame claim was added.
- **Changed files:** Only this work-order evidence record and [`execution-log.md`](../execution-log.md:1) were changed for V2-WO-004. No application source, regression test, requirements, blueprint, validation artifact, or v1 artifact changed. The aggregate source diff remains attributable to V2-WO-001/002/003 in [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1); `git diff --stat` reports 486 added and 11 removed source lines plus the prior execution-log additions, and `git diff --check` passed.
- **Durable-document reconciliation:** Requirements `REQ-WRITING-LOOP-005` / `AC-WRITING-005.4`–`005.7`, [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:59), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:78), and [`technical-context.md`](../../../blueprints/technical-context.md:31) remain accurate. Requirements and blueprint impact are therefore `verify unchanged`; no supported integrated contract changed. Native limitations remain evidence boundaries, not new product promises.
- **Next state:** `in-review`; a fresh validator must assess this evidence package. The existing [`validation-V2-WO-004.md`](../validations/validation-V2-WO-004.md:1) remains an untouched pending placeholder and is not a validation result. Native display/window evidence is the remaining confidence blocker, but not a configured project-policy hard stop.

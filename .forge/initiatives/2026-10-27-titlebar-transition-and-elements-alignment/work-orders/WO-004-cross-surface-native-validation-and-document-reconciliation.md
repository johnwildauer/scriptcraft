# Work Order: WO-004 — Cross-Surface Native Validation and Durable-Document Reconciliation

## Metadata

- **Status:** `complete`
- **Assignee:** Cross-surface validation/reconciliation implementer (one fresh session)
- **Phase/order:** 4 of 4; depends on WO-001, WO-002, and WO-003
- **Initiative:** [`../titlebar-transition-and-elements-alignment-v1.md`](../titlebar-transition-and-elements-alignment-v1.md:1)
- **Generated filename:** `WO-004-cross-surface-native-validation-and-document-reconciliation.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-004.md`](../validations/validation-WO-004.md:1)
- **Fixes:** none
- **Implementer session/agent ID:** `implementer-fresh-wo004-cross-surface-2026-10-27T1743Z`
- **Session exception:** none

## Summary

Run the final cross-surface deterministic and bounded macOS/native checks for the viewport, unified chrome, transition geometry, and Elements closed/open alignment work. Reconcile requirements and blueprints against actual implementation evidence; make a real implementation change only if a scoped defect is found, and never use this work order as an administrative shortcut to claim missing native behavior or bypass fresh validation.

## In scope

- Compare WO-001–WO-003 implementation reports, diffs, tests, geometry ledgers, and native-smoke records against the initiative outcome.
- Run the configured full workspace checks plus focused cross-surface tests.
- Execute a bounded macOS/window-capable smoke protocol when available: title/toolbar seam and hit coverage; normal→maximize and maximized→native restore direction; outer/inner/content/clip rectangles; metric samples and native animation limitation; 128-point Elements closed centering; left-aligned popup choices; arrow/truncation/keyboard behavior.
- Reconcile each affected requirement/blueprint classification with evidence. Apply a narrowly scoped source or durable-document update only when evidence identifies a real mismatch and the change remains within this initiative.
- Record unavailable native evidence, ignored commands, stale state, or platform limits explicitly.

## Out of scope

- Administrative-only edits presented as implementation, retroactive approval, design review, validation substitution, or closeout.
- Full-screen, exact universal frame-history claims, animation-duration/interpolation promises, filename/title plumbing, broad redesign, semantic editing/persistence/export changes, or unsupported cross-platform claims.
- Repeating or rewriting prior validation artifacts; each WO still receives its own fresh validator under Forge policy.

## Requirements

Applicable wording copied verbatim from [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:90):

- **REQ-WRITING-LOOP-005:** “As an individual screenwriter, I want desktop commands in a consistent toolbar, so common project actions are discoverable without changing the semantic editing model.”
- **AC-WRITING-005.4:** “Given the current caret/focus is within a screenplay element, the Elements Item shall display that element's explicit role using the seven supported labels Scene Heading, Action, Character, Dialogue, Parenthetical, Transition, and Note; choosing a label shall apply the role through the existing transactional session operation and preserve the document on rejection.”
- **AC-WRITING-005.5:** “Given a toolbar width, Space shall consume one deterministic 72-point width, rendered normal Items shall consume their complete group width including label text measured at runtime with the active egui style/font, and Flexible Space Items shall divide the remaining width equally after the shared 4-point horizontal row-padding and 4-point inter-item-gap budget, and a narrow toolbar shall retain the leftmost fitting Items while exposing non-spacing Items through a far-right `>>` text menu. Action entries and Elements role choices in that menu shall remain executable without persisting toolbar layout state; a fully fitting composition shall keep its rightmost Item, including its label, inside the actual clipped toolbar content right edge after panel, frame, and viewport insets are applied. Items with an approved typed width override, including the Elements control, shall use that same resolved width for allocation and rendering without changing the shared padding, gap, flexible-space, overflow, or clipped-edge budget.”
- **AC-WRITING-005.6:** “Given the desktop app is running, a dedicated compact title strip shall display centered bold `ScriptCraft` placeholder text above the toolbar. A double-click shall request maximize/unmaximize only when the viewport's maximized state is known; unavailable or ignored state shall use a bounded no-op/native fallback rather than an unsafe or exact-restoration claim that the platform cannot support.”
- **AC-WRITING-005.7:** “Toolbar action controls and the Elements control/label surface shall request the standard arrow cursor, while the existing whole-document editing response shall request the text/I-beam cursor. Decorative labels and painted page rectangles shall not create competing cursor owners.”

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:35) — shell ownership, viewport presentation, toolbar/Elements behavior, and single document response must remain coherent.
- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:100) — installed eframe viewport limitations and title-strip contract.
- [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:78) — desktop presentation consumes explicit semantic roles and fixed-profile layout.
- [`technical-context.md`](../../../blueprints/technical-context.md:31) — viewport changes remain presentation-only over the core/layout boundary.

## Source/API findings

- The integrated source surfaces are [`title_strip()`](../../../../apps/scriptcraft/src/main.rs:1675), [`update()`](../../../../apps/scriptcraft/src/main.rs:1697), [`elements_toolbar_item()`](../../../../apps/scriptcraft/src/main.rs:689), [`live_document_surface()`](../../../../apps/scriptcraft/src/main.rs:1080), [`presentation_geometry()`](../../../../apps/scriptcraft/src/main.rs:1509), and [`native_options()`](../../../../apps/scriptcraft/src/main.rs:1763).
- The installed [`ViewportInfo`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.29.1/src/data/input.rs:194) and [`egui_winit::update_viewport_info()`](../../../../.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-winit-0.29.1/src/lib.rs:943) findings bound native state and rectangle evidence.
- [`TOOLBAR_ELEMENTS_WIDTH`](../../../../apps/scriptcraft/src/main.rs:26) must remain the observed 128-point baseline unless WO-003 has explicit measurement evidence and separate approval for any change; this WO is not permission to undo it.

## Implementation plan

1. Assemble a traceability matrix from each prior WO’s acceptance evidence to the four applicable acceptance criteria and desktop blueprint clauses.
2. Run deterministic full-workspace and focused cross-surface checks; compare actual rectangles/metrics and inspect the comparable diff.
3. Run the bounded macOS smoke protocol if available and record unavailable/ignored behavior without upgrading it to success.
4. If a real scoped defect is found, implement the smallest correction, add a deterministic regression, and repeat affected checks; otherwise make no source change.
5. Apply only evidence-backed durable-document updates or explicitly record `verify unchanged`/`not applicable` with rationale; leave design review, human approval, implementation, and validation lifecycle gates pending after authoring.

## Files and systems

- **Create:** no application source by default; no design-review artifact; validation evidence belongs to the later validator, not this WO authoring session.
- **Update:** source only if a real defect is found; affected requirement/blueprint documents only when their classification requires an authorized durable correction; this WO and execution evidence.
- **Avoid changing:** unrelated requirements/blueprints, prior closed initiative history, filename plumbing, full-screen/native animation promises, and application behavior for administrative convenience.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; focused viewport/titlebar/Elements tests; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`.
- **Deterministic acceptance:** all prior WO tests remain green; cross-surface ledgers show no title/toolbar seam or ownership regression; viewport commands are safe for known/unknown state; Elements is centered only when closed at 128 points and popup choices remain left-aligned; document cursor ownership remains isolated.
- **Manual/native-smoke:** On macOS, record OS/backend/window configuration, command/state direction, outer/inner/content/clip rectangles, seam observations, metric samples, restore result, Elements closed/open alignment, arrow/truncation/keyboard behavior, and unavailable limitations. No exact frame-history, full-screen, animation-duration, or universal pixel claim.
- **Acceptance test:** Given the completed three implementation boundaries, when the final checks run, then the evidence supports the combined desktop-shell contract or identifies a real bounded correction; absent native access is recorded as a limitation and does not become a false pass.

## Durable-document impact

- **Requirements impact:** `verify unchanged` by default; `update` only if implementation evidence changes an acceptance contract. Check REQ-WRITING-LOOP-005 / AC-WRITING-005.4–005.7 explicitly.
- **Blueprint impact:** `verify unchanged` by default; `update` only if the integrated desktop-shell behavior differs from [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:59) or the feature composition.
- **Affected documents/IDs:** REQ-WRITING-LOOP-005 and AC-WRITING-005.4–005.7; desktop container and feature blueprints; exact selection is evidence-dependent.
- **Rationale and evidence:** This is a reconciliation boundary, not permission to manufacture a documentation delta. The closed parent, current source, installed API, prior WOs, and actual implementation diff must be compared before classification.
- **Authorized documentation changes:** Only minimal evidence-backed wording changes for unified surface/closed-only Elements centering or an actually supported native contract; preserve all stated limitations.

## Dependencies, risks, and handoff

- **Dependencies:** WO-001, WO-002, and WO-003 must each be implemented and independently validated before this work order is dispatched; project policy requires a fresh validator for this WO too.
- **Risks:** native smoke may remain unavailable; OS-owned animation may prevent universal metric claims; stale macOS state may make command observations incomplete; administrative reconciliation may conceal a real defect if source comparison is skipped.
- **Handoff:** leave a traceable final evidence package for design review and human approval; do not advance those gates in this authoring session.

## Completion checklist

- [x] Implementation/reconciliation is complete within scope, with no administrative shortcut.
- [x] Tests/checks and bounded native evidence have been recorded.
- [x] Durable-document impact classification is accurate against the final diff.
- [x] Execution log is appended with fresh dispatch/completion identifiers.
- [x] Work order is `in-review`; `complete` requires fresh validation.

## Implementation and reconciliation evidence

- **Dispatch/completion session:** `implementer-fresh-wo004-cross-surface-2026-10-27T1743Z`; fresh implementer session for WO-004 only. No session-separation exception was used.
- **Branch/status evidence:** `git status --short --branch` before checks and after cleanup confirmed `dev/2026-10-27-titlebar-transition-and-elements-alignment`. The pre-existing modified [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1), [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:1), and untracked initiative evidence were preserved. No reset, checkout, commit, or destructive Git action was performed.
- **Cross-surface source/diff reconciliation:** Compared the integrated source and working-tree diff against WO-001/002/003, their validations, the design review, requirements, and desktop/feature/technical blueprints. No new mismatch or scoped defect was evidenced. No speculative source change, regression test, requirements change, or additional blueprint change was made in WO-004.
- **Deterministic seam/geometry evidence:** `shared_chrome_geometry_has_no_gap_and_full_width_title_hit_region` passed. For the tested panel `(12,4)-(812,86)`, the title rectangle is `(12,4)-(812,34)` and toolbar rectangle `(12,34)-(812,86)`: seam gap `0.0` points, title edge coverage `800.0` points, and toolbar/panel boundary delta `0.0` points, within the `0.5`-point tolerance. The shared frame test passed with zero inner/outer margin and `Stroke::NONE`.
- **Deterministic transition-metric evidence:** Normal `800.0` and maximized-equivalent `1400.0` available widths both resolve to page width `576.0`, scale `0.941176`, and font size `11.294117` for the tested `612.0` paper width and `12.0` base font; deltas are `0.0`, within the `0.01` metric tolerance. The `400.0` narrow-width test passed as the documented bounded width-constrained case. No animation-duration, interpolation, progress, pixel-freeze, or universal native metric claim was added.
- **Deterministic Elements/alignment evidence:** `TOOLBAR_ELEMENTS_WIDTH` remains `128.0`. The closed text region is `(14,4)-(118,28)`, width `104.0`; the sample centered galley starts at `x=46.0`. The stock arrow region remains separated by `4.0` points (`text right=118.0`, arrow left=`122.0`). Long-label truncation/containment and closed-only versus stock left-aligned popup policy tests passed.
- **Viewport/cursor/width evidence:** Known-state tests passed for `Some(false) → Maximized(true)`, `Some(true) → Maximized(false)`, and `None → no command`; no full-screen or app-owned frame-history behavior exists. Cursor tests passed for standard arrow controls and text document surface ownership. Toolbar shared-edge, wide/exact-fit, narrow overflow, 128-point Elements, keyboard/open, role-routing, and rejection-preservation tests remained green.
- **Exact automated checks:** `cargo fmt --all -- --check`; focused `title_strip`, `chrome`, `presentation_metrics`, `elements`, `cursor`, and `toolbar_shared` tests; `cargo check --workspace`; `cargo test --workspace` (75 app tests, 45 core tests, integration suites, and doc-tests passed); `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; and `git diff --check` all exited 0. The first native-smoke wrapper attempt failed with a Python command-string syntax error; it was corrected and rerun, so this is recorded as a tooling attempt, not a product failure.
- **Native/window-capable protocol:** The macOS process launched and remained alive for the bounded 8-second `cargo run -p scriptcraft` attempt, but this session had no usable display (`DISPLAY` and `WAYLAND_DISPLAY` unset). A direct window query returned no visible windows and `screencapture` reported `could not create image from display`; no click/double-click, native state/rectangle, seam-pixel, transition-frame, restore, closed/open popup, cursor, or keyboard observation could be made. Native smoke is therefore **unavailable/incomplete**, not a pass claim; no OS/backend or native metric evidence is asserted. Project policy has no required manual/native gate, so this remains a warning rather than a blocker.
- **Durable-document reconciliation:** Requirements impact is `verify unchanged` for REQ-WRITING-LOOP-005 / AC-WRITING-005.4–005.7: existing wording matches the integrated tested behavior and bounded native limitations. Blueprint impact is `verify unchanged`: [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:59) already records the unified surface, viewport boundary, 128-point closed-only Elements behavior, popup alignment, cursor ownership, and presentation-only scale; feature and technical blueprints remain consistent. No durable-document update was authorized by evidence.
- **Changed files:** No application source or test files changed in WO-004. No requirements or blueprint files changed in WO-004. Only this work order and [`execution-log.md`](../execution-log.md:1) were updated with the evidence record; the pre-existing source/blueprint diff remains attributable to WO-001–WO-003.
- **Comparable diff:** Final `git status --short --branch`, `git diff --stat`, source/blueprint inspection, and `git diff --check` show the same two tracked implementation/document files plus the untracked initiative evidence directory; no WO-004 runtime delta is present. The final diff remains comparable to the recorded `master` base and prior WO validation evidence.
- **Blockers and next state:** No scoped defect, required-policy failure, or comparison blocker was found. Native visual/window observation is unavailable and remains an explicit warning. Fresh validation passed; WO-004 is `complete`. See [`validation-WO-004.md`](../validations/validation-WO-004.md:1).

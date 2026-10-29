# Work Order: WO-008 — Remove Document Title Strip

## Metadata

- **Status:** `complete` — limited aggregate directional-alignment check passed; this is not comprehensive code validation.
- **Assignee:** Dedicated fresh coding implementer, separate from WO-005–WO-007; actual host dispatch ID `01a18632-ede4-70b7-a5d3-2046acd2c618` (reconciled from the user's instruction; no internal session ID is inferred).
- **Phase/order:** 4 of 4 in the user-directed follow-on batch; final product WO.
- **Initiative:** [`status-bar-v2.md`](../status-bar-v2.md)
- **Dependencies:** WO-007 implementation handoff received; work sequentially after it.
- **Fixes:** none; new scoped change.

## Summary

Remove the specific visible `Untitled Screenplay` project-title heading and its separator directly below the toolbar. Remove only this redundant UI strip; retain project title metadata, title-page/document content, toolbar, editor, and all other UI.

## In scope

- Inspect app composition in [`ScriptCraftApp::update()`](../../../../apps/scriptcraft/src/main.rs:1132); the exact target is the central-panel `ui.heading(&self.session.project().metadata.title)` plus immediately following `ui.separator()`.
- Remove that heading/strip and separator while continuing to render the live document surface in the existing central panel without a blank reserved row.
- Prove project metadata title, title-page elements/document data, save/open behavior, and actual document title-page rendering are untouched.
- Add focused rendered geometry/content regressions proving no duplicate project-title heading appears between toolbar and editor and other UI remains.
- Update a durable requirement/blueprint only if inspection shows it promises this redundant strip; otherwise record `verify unchanged` with evidence.

## Out of scope

- Renaming, clearing, hiding, or changing project title metadata, save-dialog naming, project files, title-page content, PDF/text exports, or native window title.
- Removing toolbar, changing native OS title bar, changing title-page content, editor, page controls, status bar, fonts, or any other UI element.
- Changing core, schema, persistence, pagination, source mapping, PDF/export, or unrelated layout behavior.

## Requirements and durable-document impact

- **Requirement:** Verify [`AC-WRITING-001.1`](../../../requirements/features/screenplay-writing-loop.md:44) remains satisfied: project title-page metadata and editable starting element remain. No requirement update is expected unless a current criterion explicitly specifies the redundant heading.
- **Blueprint:** Verify [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:1) and relevant editor/title-page ownership descriptions. Update only if they explicitly require a separate project-title strip; retain native titlebar/toolbar contracts.

## Implementation files

- **Allowed product path:** [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1184) and focused app tests in that module only.
- **Allowed durable records:** strictly necessary requirement/blueprint correction, this WO, [`status-bar-v2.md`](../status-bar-v2.md), and [`execution-log.md`](../execution-log.md).
- **Avoid changing:** toolbar/titlebar ownership/controls, document model/core, title-page data, save/open/export, font assets, status bar except integrated test setup, WO-004/historical evidence, `.DS_Store`, and unrelated paths.

## Measurable acceptance and focused regressions

1. In an actual central-panel/app render, no project-metadata heading or separator strip appears between toolbar and first document surface; freed vertical space is available to the document surface (no empty spacer reserves the removed strip height).
2. Default new project `metadata.title` remains exactly `Untitled Screenplay` and remains unchanged after save/open. Existing title-page content/layout and PDF/text export results remain unchanged.
3. Toolbar geometry/actions, status/page/zoom controls, editor response/input/focus/caret behavior, and other visible UI remain present and unchanged by this localized removal.
4. Regression distinguishes the removed duplicate UI heading from title text legitimately rendered as title-page/document content and from native window/titlebar text.

## Implementation evidence — 2026-10-29

- **State/dispatch:** WO-008 transitions `in-progress` → `in-review` after the scoped change and required gates. WO-005/006/007 remain `in-review`; the user's sole aggregate alignment check remains pending. This implementer's actual host dispatch ID is pending orchestrator reconciliation; no internal session/agent ID is available or inferred. No per-WO validator or additional review was dispatched.
- **Aggregate reconciliation:** The actual WO-008 host dispatch ID `01a18632-ede4-70b7-a5d3-2046acd2c618` is reconciled. The single limited aggregate directional-alignment check is recorded in [`validation-aggregate-WO-005-through-WO-008.md`](../validations/validation-aggregate-WO-005-through-WO-008.md); this WO is complete for that limited purpose only, not comprehensively validated.
- **Implementation:** `apps/scriptcraft/src/main.rs` now shares the real toolbar → status → central-panel composition through `show_application_panels()`, called by the production app update path and the focused headless render regression. The central panel renders `live_document_surface()` directly; only the project-metadata heading and following separator were removed. Project metadata, document/title-page elements, toolbar, status controls, editor, viewport/zoom/input/focus paths, and persistence code were not changed.
- **Rendered regression:** `rendered_application_omits_project_title_strip_and_keeps_title_page` round-trips a fixture project through JSON deserialize/open replacement, then inspects actual emitted egui text and editor geometry in the shared application panel composition. It proves the `Untitled Screenplay` metadata is retained but not duplicated in the UI, title-page content remains rendered, toolbar `New` and status `100%` remain rendered, and the editor begins within the central panel's normal inset (<20 points) without a reserved title row. Existing named save/open and failed-open regressions also pass in the complete app suite.
- **Durable documents:** Requirement `AC-WRITING-001.1` was inspected; its title-page metadata and editable starting-element contract remains satisfied and unchanged. The desktop blueprint was inspected; it retains native titlebar/toolbar and title-page ownership contracts and does not require this redundant project-title strip. Both are `verify unchanged`; neither file was edited.
- **Command attempts/results:** Initial `cargo fmt --all -- --check && cargo test -p scriptcraft rendered_application_omits_project_title_strip_and_keeps_title_page -- --nocapture` exited 1 at one rustfmt-only line wrap; the exact formatting was corrected in `main.rs`. Final rerun exited 0: format check passed and focused test passed (1/1). `cargo test -p scriptcraft` passed (115/115). `cargo check --workspace`, `cargo test --workspace` (197 total: 115 app, 45 core unit, 37 integration; 0 failed), `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` all exited 0. These workspace gates were run together in a successful `&&` chain.
- **Native smoke/limitations:** Native desktop visual/input smoke was unavailable and not performed. The actual egui render regression is headless evidence, not a native-window smoke pass. No native titlebar, OS input, or visual smoke outcome is claimed.
- **Changed-path/preservation boundary:** WO-008 product/test change is limited to `apps/scriptcraft/src/main.rs`. WO-008 state/evidence and the WO-007 dispatch reconciliation are recorded in this WO, [`WO-007-courier-prime-default-document-font.md`](WO-007-courier-prime-default-document-font.md), [`status-bar-v2.md`](../status-bar-v2.md), and [`execution-log.md`](../execution-log.md). Requirements, desktop blueprint, core/persistence/export, status-bar module, toolbar module, all supplied font assets/licenses, and other prior work remain untouched by WO-008. No staging, commit, reset, stash, merge, or cleanup was performed.

## Checks and handoff evidence

- Run focused app tests for startup title metadata, central-panel/rendered heading, title-page content, and existing toolbar/status/editor behavior.
- Run project gates: `cargo fmt --all -- --check`; `cargo check --workspace`; `cargo test --workspace`; `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `git diff --check`.
- Native visual smoke is recommended but optional under the manifest; report unavailable if not performed. Record exact paths, test results, durable-document verification, and limitations.
- **Acceptance test:** Given a default or saved project with a title, when the main window renders, the redundant title strip directly below the toolbar is absent, the editor begins without that heading/separator row, and underlying title metadata/title-page/export data remain intact.

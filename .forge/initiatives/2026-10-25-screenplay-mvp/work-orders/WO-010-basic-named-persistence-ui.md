# Work Order: WO-010 — Basic Named Persistence UI

## Metadata

- **Status:** `complete`
- **Assignee:** Desktop persistence-UI implementer (one fresh session)
- **Phase/order:** 10 of 10; depends on WO-006, WO-009, and WO-008
- **Initiative:** [`../2026-10-25-screenplay-mvp-v1.md`](../2026-10-25-screenplay-mvp-v1.md)
- **Generated filename:** `WO-010-basic-named-persistence-ui.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-010.md`](../validations/validation-WO-010.md) — passed independent fresh-context validation
- **Fixes:** `none`
- **Implementer session/agent ID:** `code-session-2026-10-25-wo-010`
- **Session exception:** `none`

## Summary

Replace hard-coded `scriptcraft.json` Open/Save behavior with basic user-selected
and named Open, Save, and Save As actions integrated into the unified live
document surface. Keep the existing bounded native persistence and recovery core
contracts unchanged; recovery remains outside this UI scope.

## In scope

- A current native-project path that is set after Open or Save As and is shown in
  status/path presentation.
- Basic user-selected Open, Save, and Save As actions using the app's native file
  selection boundary; Save uses the current named path when available and offers
  Save As when no path exists.
- Successful open replacement, successful atomic save, dirty-state behavior, and
  visible bounded failure/status handling using the existing persistence APIs.
- Tests or seams proving Open/Save/Save As use the selected path and do not mark a
  session clean after a failed save or replace.
- Removal of hard-coded native-project UI dependence on `scriptcraft.json`.

## Out of scope

- Recovery UI, recovery browsing, automatic recovery replacement, or changes to
  recovery discovery/rotation. Recovery mechanisms remain outside the UI.
- Persistence schema/migrations, atomic-save implementation, exports, export path
  redesign, recent-files lists, preferences, cloud/network storage, or project
  management.
- Any second editor surface or continuous/page-preview editing mode.

## Requirements

- **REQ-WRITING-LOOP-003:** “As an individual screenwriter, I want to save, reopen, and recover my project without losing the last valid version, so that an interrupted session does not destroy my draft.”
- **AC-WRITING-003.1:** Given a valid project, the system shall serialize a versioned native representation containing semantic content, stable IDs, metadata, and page-profile inputs, while excluding transient caret/layout/undo state from required content.
- **AC-WRITING-003.2:** Given a save preparation or replacement failure, the system shall preserve the prior valid destination and retain an unsaved/dirty indication.
- **AC-WRITING-003.3:** Given a successfully saved native project, reopening it shall preserve semantic roles, text, stable IDs, metadata, and reproducible pagination inputs.
- **AC-WRITING-003.4:** Given a newer valid bounded recovery snapshot after interruption, the system shall offer it as a candidate and shall not silently overwrite the named project.

## Blueprints

- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md) — the host owns file selection and the current named path while delegating bytes and atomic-save semantics to core.
- [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md) — native persistence remains the canonical source; recovery remains an explicit non-UI/core boundary for this narrow correction.

## Implementation plan

1. Add the smallest app-local file-selection/path state needed for Open, Save,
   and Save As, removing the fixed `scriptcraft.json` project path.
2. Route selected paths through existing `deserialize`, `atomic_save`, session
   replacement, and `mark_saved` behavior; preserve failure safety and dirty state.
3. Add focused path/status/error tests and manual evidence for named Open, Save,
   Save As, and failed-save preservation; verify no recovery UI is added.

## Files and systems

- **Create:** App-local persistence UI adapter/tests only if needed.
- **Update:** [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs), app manifest/lockfile only if required for native file selection, this WO, execution log, and the desktop blueprint with verified path-ownership facts.
- **Avoid changing:** `crates/scriptcraft-core/src/persistence.rs`, native schema, recovery APIs, editing/layout/export behavior, and the unified editor surface except for integration hooks.

## Validation and E2E acceptance tests

- **Automated checks:** `cargo fmt --all -- --check`; `cargo check --workspace`; `cargo build --workspace`; `cargo test --workspace -- --nocapture`; `cargo clippy --workspace --all-targets -- -D warnings`; focused app path/error tests.
- **Manual/external checks:** Open a named native project, edit and Save, Save As to a second named path, reopen both paths, confirm the original remains unchanged, and induce/observe a save failure while confirming the old file and dirty state remain safe. Confirm there is no recovery button/dialog in this UI scope.
- **Acceptance test:** Given a dirty project in the unified live editor, when the writer uses user-selected Open, Save, and Save As, then native bytes are read/written only at the selected/current named path, successful operations update path/clean status correctly, failed operations preserve valid document/file state, and recovery remains outside the UI.

## Durable-document impact

- **Requirements impact:** `verify unchanged`
- **Blueprint impact:** `update`
- **Affected documents/IDs:** `AC-WRITING-003.1`–`003.4`, [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md)
- **Rationale and evidence:** The native persistence core passed WO-006/WO-008 validation, but the current app hard-codes [`native_path`](../../../apps/scriptcraft/src/main.rs:18-44) to `scriptcraft.json` and calls Open/Save against it at [`main.rs`](../../../apps/scriptcraft/src/main.rs:46-77). This WO narrows the UI correction to named path selection and explicitly leaves recovery outside the UI.
- **Authorized documentation changes:** Record app-owned selected/current path behavior and the reduced basic-persistence UI scope; do not alter schema or recovery requirements.

## Completion checklist

- [x] Implementation is complete within scope.
- [x] Tests/checks and required gates have evidence.
- [x] Durable-document impact classification is accurate against the diff, and authorized documentation updates are applied.
- [x] Execution log is appended, including dispatch and completion session/agent identifiers.
- [x] Work order is `in-review` with recorded evidence; `complete` comes only from a passed fresh validation in [`validation-WO-010.md`](../validations/validation-WO-010.md).

## Implementation evidence — 2026-10-25

- **Changed files:** [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs), [`apps/scriptcraft/Cargo.toml`](../../../apps/scriptcraft/Cargo.toml), workspace [`Cargo.lock`](../../../Cargo.lock), this work order, [`execution-log.md`](../execution-log.md), and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md).
- **Host boundary:** `rfd::FileDialog` supplies native Open/Save As selection; the app stores an optional current named path. Save uses that path when present and delegates bytes/atomic replacement to existing core persistence.
- **Safety behavior:** successful Open replaces the session and path only after read/deserialize/session replacement succeeds; successful Save updates clean state/path only after atomic replacement. Failed Open/Save preserve the prior path and session state. Open is refused while dirty with an explicit save-first status.
- **Focused evidence:** 8 app tests pass, including named path routing, successful Open/Save state transitions, failed-save file/path/dirty preservation, and failed-open non-replacement. Recovery remains absent from the UI.
- **Validation status:** Implementer gates passed; no independent validation claim is made. WO remains `in-review` pending fresh validation.

## Independent validation — 2026-10-25

- **Validator/session:** Independent fresh-context validator, `validation-session-2026-10-25-wo-010`.
- **Result:** `PASS`. Named Open, Save, and Save As path routing; successful clean/path transitions; dirty/open refusal; non-destructive failed save/open behavior; absence of recovery UI and stale persistence messaging; and preservation of the unified WO-009 editor all aligned with scope. Format, check, build, test, clippy, focused path/error tests, and bounded launch evidence were recorded in the linked validation artifact.
- **Evidence:** [`validation-WO-010.md`](../validations/validation-WO-010.md).
- **Required next state:** WO-010 is `complete`; no lettered fix work order is required.

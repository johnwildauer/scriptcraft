# WO-002 — Diagnostic status command and routine-message removal

- **Date:** 2026-11-07
- **Status:** Implementation submitted; user review, validation, and acceptance pending
- **Initiative branch:** `dev/2026-11-06-ui-indicators-platform-shell`
- **Scope:** WO-002 only. WO-001 changes were already present and remain preserved; WO-003–WO-007 were not implemented.
- **Validation owner:** User under the authoritative repository policy in [project-manifest.md](../../../.forge/project-manifest.md).

## Result

Added the codec-independent, typed `ui.status.set` command to the command ID vocabulary, Serde command
contract, descriptor catalog, and command-to-ID mapping. `DiagnosticMessage` rejects empty values,
control/line-separator characters, and values over 512 UTF-8 bytes at construction/deserialization.
The optional payload sets or clears one desktop diagnostic message; the headless semantic executor
rejects the command as unsupported without touching its session.

The desktop dispatcher is the sole writer of diagnostic state. Report and clear producers enqueue
typed commands with `InternalFollowUp` origin. The dispatcher flushes prior queued reports before a
new activation and flushes reports generated during a dispatch afterward; FIFO order is retained. A
drain guard prevents nested dispatch from recursively draining the queue. `ui.status.set` remains
available during migration modal state. This keeps dispatch-origin reporting ordered without allowing
diagnostic effects to mutate screenplay content, selection, history, dirty state, persistence, layout,
or exports.

Routine editor, selection, clipboard, document, export, and scheduled-refresh success/development text
was removed. Retained errors and warnings go only to the existing Diagnostics disclosure. Save
failure details preserve the transactional error category and safe detail text, use an explicit
`FDX_SAVE_FAILED` prefix, state that destination/live document remain unchanged, and continue avoiding
screenplay contents and filesystem paths. Other errors use bounded privacy-safe codes/details and
preserve unchanged-document semantics. The existing migration confirmation/warning dialog is
unchanged. The status center has an interim fixed **Ready** projection until WO-004 supplies live
cursor and modifier hints. Diagnostics remains source-disabled by default. When disabled, errors remain
retained but are intentionally hidden; the Diagnostics row, body, focus/accessibility nodes, and
placeholder allocation are not produced, and errors do not fall back into the center.

The authoritative command catalog and descriptor inventory are now 42 entries (formerly 41). No
dependency or lockfile change was made.

## Changed paths

- `crates/scriptcraft-commands/src/lib.rs` — stable ID, 42-entry inventory, bounded typed payload,
  command variant, `id()` mapping, and descriptor.
- `crates/scriptcraft-commands/tests/editor_executor.rs` — authored catalog/bounds/headless-rejection
  coverage.
- `apps/scriptcraft/src/app_commands.rs` — desktop command handler, modal availability, ordered
  diagnostic command queue/drain, routine success-message removal, and authored desktop command tests.
- `apps/scriptcraft/src/main.rs` — diagnostic state, save/open/export/refresh reporting, clear routing,
  error health details, and interim center projection wiring; pre-existing WO-001 title code retained.
- `apps/scriptcraft/src/script_view.rs` — layout errors are routed to Diagnostics, not inline or center
  text; routine editing-success labels removed.
- `apps/scriptcraft/src/status_bar.rs` — Ready center projection and opt-in diagnostic-message row;
  authored default gate/Ready coverage.
- `.forge/requirements/features/screenplay-writing-loop.md` — cursor-only center, explicitly hidden
  diagnostics-off behavior, typed command and 42-ID inventory requirements.
- `.forge/blueprints/technical-context.md` — command ownership/order, bounded payload, hidden default
  state, and dispatcher behavior.
- `.forge/project-manifest.md` — current submission, branch/work preservation, command count, and
  unvalidated evidence record.
- `.plan/2026-11-06-ui-indicators-platform-shell/ui-indicators-platform-shell-v1.md` — WO-002
  implementation assessment and updated initiative status.

The existing WO-001 handoff and all other pre-existing tracked/untracked work were preserved. The
branch was not changed; no stash, reset, commit, merge, or overwrite was performed.

## Authored coverage and limitations

Authored but **not run**:

- Command inventory count and exact descriptor coverage at 42 entries.
- Valid bounded diagnostic payload construction, oversized and multiline rejection.
- Headless executor rejection with unchanged editor session.
- Desktop dispatcher set, replace, and clear behavior.
- FIFO report ordering, migration-modal command availability, Ready projection, and Diagnostics-off
  default.

No concrete Serde codec/round-trip test was added because the command crate deliberately leaves codec
selection to its host and this WO adds no serialization dependency. The custom `Deserialize`
implementation applies the same message validator, but its serialized wire round-trip remains for
user-owned validation. The actual Diagnostics-enabled rendering path, save/open/clipboard error
scenarios, platform interactions, and source compilation also remain unverified.

No tests, build, check, lint, formatting command, launch, manual verification, review, agent validator,
or other validation was run. This is an implementation submission only, not evidence of compilation,
behavior, or acceptance. No blocker was identified from the source work; the codec round-trip and all
runtime checks remain explicit user-validation items.

## User-owned acceptance focus

1. Verify a 42-entry ID/descriptor catalog and command serialization/deserialization bounds for
   absent, valid, empty, multiline, control-character, and oversized payloads.
2. Verify direct set, replacement, clear, queued order, and reporting/clearing while migration modal
   is active; confirm dispatch-origin effects do not recurse or reorder.
3. Exercise typing/editing, selection/navigation, copy/cut/paste, New/Open/Save/Save As/PDF, scheduled
   refresh, and failures. Confirm no routine text displaces center guidance and failures preserve
   session/document safety.
4. With Diagnostics off, confirm it produces no disclosure row/body/focus/accessibility node and
   errors remain hidden rather than leaking into the center. Enable it only by the documented source
   gate and rebuild/relaunch to confirm privacy-safe diagnostic details render there.
5. Confirm current center says **Ready** pending WO-004; verify native title lifecycle and existing
   migration modal warnings remain unchanged.

WO-003 through WO-007 remain pending. Do not treat this handoff as validation evidence.

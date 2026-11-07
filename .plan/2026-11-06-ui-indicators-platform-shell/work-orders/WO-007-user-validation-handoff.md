# WO-007 — Consolidated user-validation handoff

- **Initiative:** [UI indicators and platform shell v1](../ui-indicators-platform-shell-v1.md)
- **Validation owner:** User only, per [authoritative manifest policy](../../../.forge/project-manifest.md).
- **Implementation state:** WO-001–WO-006 are submissions, not accepted or validated. All reported authored coverage is unrun. This handoff records no results.
- **Agent restrictions:** Do not dispatch/perform review or validation and do not run any command below. Commands are listed solely for the user.

## User-run workspace command inventory

Run these only if desired/required by the user's validation plan. These are proposed commands, **not results**; no agent ran them for this initiative.

```text
cargo fmt --all -- --check
cargo check --workspace
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Keep each outcome, toolchain/target, and any warnings/failures as user evidence. These commands cannot establish native window, input, menu, accessibility, or cross-OS behavior. Do not infer pass/fail from the presence of authored tests. WO-002 has no concrete Serde codec round-trip test; record the codec exercised and whether that gap remains a blocker to acceptance.

## Native window title lifecycle — macOS and Windows

On each available OS, exercise and record:

1. Fresh/unsaved startup and successful New show **Untitled Screenplay**.
2. Successful Open shows only the active final filename component, including extension; test spaces and Unicode.
3. Successful first Save As, subsequent Save As to a new filename, and ordinary Save update the native title when the active filename changes. Same-name/repeated frame behavior should not create visible duplication.
4. Failed/cancelled Open, Save, and Save As preserve the prior path/title and document state.
5. Long filenames, title clipping/placement, decorations, and unchanged toolbar/content layout.
6. Native title positioning is OS-owned. Centering is not a product guarantee; record actual macOS/Windows appearance without requiring app-owned centering.

## Native window and menu startup/failure lifecycle

- On macOS `auto` and `macos`, confirm native Application and all shared catalog menus only; no in-window strip. Confirm native menu input reaches one command/accelerator owner.
- On Windows `auto` and `windows`, confirm the complete in-window catalog only, no native controller, and one in-window accelerator owner.
- On macOS `--menu-platform=windows`, confirm only the egui strip is installed and native construction is skipped. Confirm Windows-style menu labels/annotations show **Ctrl**, but actual macOS menu activation remains **Command-based**; editor/status shortcuts remain Command-based too. Check one physical activation produces no duplicate command.
- On Windows and another non-macOS target, `--menu-platform=macos` must reject before window/menu launch. On other-target `auto`, confirm no menu; explicit `windows` selects only the in-window renderer.
- Exercise missing `=value`, empty, unknown/case-mismatched value, duplicate option, unknown switch, positional argument, and (where available) non-UTF-8 argument. Record process exit/output; handoff reports usage errors with exit code 2 before eframe launch.
- Review selected native-menu installation failure: reported behavior is contextual startup failure/abort, not fallback. Actual eframe/launcher error visibility is an explicit open observation, not a claimed result.
- Check native menu labels/availability/placeholders, command FIFO, popup switching/navigation/dismissal, editor-focus restoration, command ordering, and no duplicate accelerator execution.
- Confirm the preview does not alter native dialogs, clipboard, notifications, window controls, page/document-point conversion, native scale, editing semantics, or actual host modifier behavior.

## Status guidance, all roles, focus, selection, and IME

- Exercise all 17 roles on macOS and Windows, empty and nonempty elements, at end/interior positions, and every resolved Enter/Tab hint. Check omitted `NoOp`/`Unavailable` targets, same-role General Tab display versus consumed no-op, and the Parenthetical delimiter/repair/generated-pair cases.
- At eligible collapsed element ends, compare visible hints with actual Enter/Tab outcomes; confirm a hint does not promise a later capacity/preservation transaction will succeed.
- Verify base and extended role/digit lists against the existing bindings, Summary-first extended presentation, and exact chords: macOS Command / Command+Control; Windows Control / Control+Shift. Try release, wrong platform modifiers, Option/Alt, and unsupported extra modifiers.
- Verify focus loss, interior caret, forward/reversed noncollapsed selection, IME preedit/commit/current-frame IME events, and migration modal produce the specified Ready or held-choice priority. Confirm modifier-only changes repaint without editor command or menu-queue activation.
- Inspect migration-modal Ready footer: noninteractive, no status controls or Diagnostics row.

## Status layout, diagnostics, errors, and regression safety

- Inspect status at default, long-content, and narrow window widths, including 1200, 480, and 320 logical-point cases where practical. Verify one row, clipping rather than wrapping/overflow, page counter and five right controls stay bounded/nonoverlapping, and modifier-list accessible text is complete even if its visible text clips.
- Check long/unicode labels and clipping, default appearance and Day/Night/Midnight, accessible names/focus, and stable toolbar/editor visibility. Record any list usability issue; the reported font floor does not guarantee the entire visual list fits.
- Confirm Diagnostics is intentionally disabled by default: retained errors assigned there are hidden; no disclosure row/body/placeholder/focus/accessibility node appears, and no error leaks into the center. This is the specified visibility tradeoff.
- If the user chooses to enable Diagnostics through its manual source gate for their own inspection, rebuild/relaunch and verify privacy-safe entries there, set/replace/clear ordering, modal reporting, and no document text/source/XML/IDs/unnecessary paths. Restore the production-off gate after that inspection if desired. This is user work, not an agent edit.
- Exercise save/open/clipboard/edit/export/scheduled-refresh failures, including Save and Save As categories. Check prior destination/live document/session safety, unchanged-document wording, title preservation, and whether errors assigned to Diagnostics are unavailable when the gate is off. Existing migration confirmation/warning behavior is separate and should remain unchanged.
- Confirm typing, selection, navigation, undo/redo, clipboard, New/Open/Save/Save As/PDF, page/zoom, toolbar/status controls, and menu actions retain existing command ordering and document/persistence/export semantics. Disabled menu placeholders must remain inert. New dirty-session consent discrepancies are not claimed fixed by these work orders; report observed behavior without attributing a fix.

## Known coverage gap and pending acceptance record

WO-002's handoff explicitly states that no concrete Serde codec round-trip test was added; the command crate leaves codec selection to its host. The custom deserializer uses the same message validator, but wire serialization/deserialization round-trip is not established. User validation must identify a concrete host codec path and evidence or state that the gap blocks acceptance pending separately authorized work. Do not invent round-trip coverage or result.

Other reported limitations to record rather than assume resolved: all compilation/tests/runtime behavior are unverified; native-menu install failure surfacing by eframe/launcher is unknown; native OS modifier delivery, rendering/accessibility, diagnostic-enabled row, and Windows runtime are unobserved; long modifier guidance may visually clip at narrow widths; title centering remains OS-owned and has no Windows guarantee. The initial handoffs reported no implementation blocker, but that is not validation evidence.

For each scenario, record host/OS/version, exact user-run command or interaction, observed output/behavior, failure and document-safety outcome, evidence location, and disposition. Keep **pending**, **not run**, and **unavailable** distinct from pass. All WO-001–WO-006 submissions remain **unaccepted** until the user supplies evidence and acceptance; this handoff does not authorize agent-side checks or claim completion by validation.

# WO-006 — Document, clipboard, modal, and native-window effects

- **Status:** Implemented; user-owned validation pending
- **Initiative:** [Hotkeys and consolidated command layer v1](../hotkey-command-layer-v1.md)
- **Depends on:** [WO-005](WO-005-selection-navigation-focus.md)
- **Validation owner:** User; no agent-run checks, reviews, or validators

## Single outcome

Complete command routing for existing document lifecycle, clipboard, migration-modal, and application-window actions through explicit host adapters, with direct-path operations available without a GUI picker.

## Scope

1. Separate `file.open.request` from direct-path `file.open`, and `file.save-as.request` from direct-path `file.save-as`. Picker adapters only obtain a path; successful selection dispatches the direct-path command through the same dispatcher.
2. Preserve `file.save` behavior: use the current native path when present, otherwise dispatch the existing Save As request. Preserve extension checks, current Save/Save As mode selection, source baseline, transactional save, atomic replacement, status/error, and failure rollback in [`main.rs`](../../../apps/scriptcraft/src/main.rs:702).
3. Route direct-path Open through existing staged source reading, content-based format selection, FDX version preflight, migration consent, session activation, restored view state, and baseline updates. A direct-path command never opens a picker.
4. Route Continue/Cancel in [`show_fdx_migration_dialog()`](../../../apps/scriptcraft/src/main.rs:1214) through `file.import.resolve` with a typed decision. Preserve pending-source ownership, dirty-session refusal, cancel behavior, and one-shot activation.
5. Keep canonical clipboard extraction in the desktop clipboard adapter. Copy writes exact selected canonical UTF-8; Cut writes first and only then dispatches forward deletion; Paste reads only on activation and dispatches text paste only for nonempty readable text. Clipboard failures do not mutate the screenplay.
6. Route File Close through the viewport effect adapter. Keep native window-state capture and imported one-shot restore as host observations/effects, not screenplay commands or semantic Project state.
7. Keep New, Open, Save, Save As, PDF export, clipboard, and Close behavior unchanged, including current fixed `scriptcraft.pdf` export destination. Do not add a PDF picker or unsaved-change confirmation in this order.
8. Record the native predefined Application-menu boundary: About, Hide, Hide Others, Show All, and Quit remain OS-owned unless existing safe host APIs can route them without new unsafe AppKit messaging. Do not claim they are ScriptCraft-dispatched commands.

## Out of scope

- No New/Open/Close dirty-session consent redesign, recovery feature, additional export path, file-format change, or native OS service reimplementation.
- No new clipboard format, asynchronous clipboard protocol, or per-frame clipboard polling.
- No broad Objective-C bridge or new unsafe-code allowance.

## Structural acceptance for user review

- Dialog cancellation produces no direct-path command and no session/filesystem mutation.
- Direct-path Open/Save As can execute without rfd and use the same core/storage operations as picker-driven activation.
- Clipboard follow-up commands preserve activation order and cannot delete/insert after a failed clipboard effect.
- Migration commands cannot bypass current source, version, dirty-state, or transaction safeguards.
- Every enabled ScriptCraft-owned document/window action uses the consolidated dispatcher; predefined native OS services are explicitly documented as the only exception.

## User-owned checks

- Exercise picker cancel/success and direct-path Open/Save/Save As, invalid extensions, missing/malformed/non-6 FDX, migration Continue/Cancel, and Save failure/conflict safety on disposable synthetic files.
- Exercise Copy/Cut/Paste with collapsed/noncollapsed/Unicode/cross-element selections, empty/nontext/unavailable clipboard, and failed Cut write.
- Verify Close and native Application-menu behavior on macOS, and record any platform-owned exception rather than inferring command dispatch.
- Run applicable workspace checks and headless direct-path scenarios.

## Durable-document impact

Update the desktop container/technical context with dialog versus direct-path commands, clipboard follow-up ordering, migration commands, and native effect ownership. Requirements impact is update for headless direct-path invocation and uniform action safety; persistence requirements remain unchanged.

## WO-006 implementation handoff — 2026-11-06

- Open and Save As now have separate picker-request and direct-path dispatcher arms. Picker adapters only select a path; a selected UTF-8 path is wrapped in the typed `DocumentPath` and dispatched as `file.open` or `file.save-as` with `InternalFollowUp` origin. Picker cancellation dispatches no direct-path command. The direct-path handlers call the existing staged Open and Save As operations without opening a picker. A selected path that cannot be represented by the typed UTF-8 path contract reports an error instead of being lossily rewritten.
- Save with an existing native path still uses `save_native` and its current Save/Save As mode selection. Without a path, it dispatches the existing `file.save-as.request` through the common dispatcher. Direct Save As retains `.fdx` extension validation and all existing transactional persistence, baseline, atomic replacement, status/error, and failure-rollback logic. Direct Open continues through staged source reading, content-based format selection, FDX version preflight, migration staging, activation, restored view state, and baseline handling. PDF export retains the fixed `scriptcraft.pdf` destination; no picker or dirty-session confirmation was added.
- Migration Continue/Cancel now dispatches `file.import.resolve` with typed `ImportResolution`. Fresh availability admits that command only while a migration is pending and keeps other commands modal-blocked. The resolver still takes ownership of the pending source once, preserves dirty-session refusal, and leaves the active document unchanged on cancellation.
- Copy retains exact canonical selection extraction and clipboard-write status/error behavior. Cut now dispatches `edit.delete` through the common dispatcher only after clipboard write success, then restores its existing successful Cut status; clipboard failure does not dispatch deletion. Paste reads only at activation and dispatches `editor.text.paste` through the same dispatcher only for nonempty readable text, retaining its successful Paste status. Empty/unavailable clipboard notices and no-mutation behavior remain in the adapter. Internal follow-ups use `InternalFollowUp` origin. Dispatcher `EffectDispatched` remains a routing outcome, never evidence that an external adapter succeeded.
- File Close and imported one-shot window-restore commands are sent by a dedicated viewport effect adapter. Native window-state capture and pending imported restore remain desktop-host observations/effects, not screenplay commands or semantic Project state. Native predefined macOS About/Hide/Hide Others/Show All/Quit remain OS-owned; no new AppKit messaging or unsafe-code allowance was added.
- Updated the technical context and AC-WRITING-012.3 to record the direct-path/dialog split, staged migration safeguard, clipboard dispatch order, viewport ownership, and unchanged document safety behavior. WO-007 view/status/timer command routing remains out of scope.
- Exact files changed: `apps/scriptcraft/src/app_commands.rs`, `apps/scriptcraft/src/main.rs`, `.forge/blueprints/technical-context.md`, `.forge/requirements/features/screenplay-writing-loop.md`, this work order, and `.plan/2026-11-06-hotkey-command-layer/hotkey-command-layer-v1.md`. No dependency or lockfile change was needed.
- Source-level evidence only: dispatcher arms distinguish request commands from path-bearing commands; dialog completion dispatches typed direct-path commands; Save's no-path path dispatches the request command; modal choice constructs `FileImportResolve`; clipboard follow-up calls enter `dispatch_command`; Close and imported restore are sent through `DesktopViewportEffects`. These source paths do not establish compilation or runtime behavior.
- Risks and user-owned validation: typed paths are UTF-8 only, so a native picker selection with a non-UTF-8 path is explicitly rejected; direct-path Open/Save As, nested follow-up outcomes, modal availability, native viewport requests, platform dialogs/clipboard, and prior dispatcher integration are not runtime-confirmed. The desktop effect result remains `EffectDispatched` even when an adapter reports a failure through its existing status/error channel. User review/validation should cover picker cancel/success and direct-path Open/Save/Save As; extension errors; missing, malformed, duplicate, current and non-current FDX version cases; Continue/Cancel with pending source, dirty-session refusal, and one-shot activation; transactional Save failure/conflict rollback and baseline mode; Copy/Cut/Paste with collapsed, noncollapsed, Unicode, cross-element, empty, nontext/unavailable clipboard, and failed Cut write; Close and imported window restore; native Application-menu ownership; and preservation of exact existing statuses/errors. No tests, builds, checks, Clippy, formatting, manual verification, review, or independent validation were run. User validation remains pending.
- WO-007 handoff: retain the typed dispatcher and adapter-result distinction. Implement only view/status/appearance/focus-mode and sprint-timer/notification command routing; do not change WO-006 document, clipboard, migration, viewport, status/error, or existing editing semantics. Preserve the exact existing document/status behavior when adding any shared result handling.

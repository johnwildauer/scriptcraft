# WO-007 — Durable contracts and user-validation handoff

- **Submission date:** 2026-11-07
- **Initiative:** [UI indicators and platform shell v1](../ui-indicators-platform-shell-v1.md)
- **Branch:** `dev/2026-11-06-ui-indicators-platform-shell` (reported by WO-001–WO-006; no branch operation in WO-007)
- **State:** Documentation submitted; implementation submissions WO-001–WO-006 remain unvalidated, unaccepted, and pending user-owned validation
- **Scope:** WO-007 only. No production source changes, features, or additional implementation were made.
- **Validation owner:** User, under the authoritative repository-wide policy in [project-manifest.md](../../../.forge/project-manifest.md).

## Reconciled current contract

This closeout records the behavior reported in the six implementation handoffs; it does not independently review or validate that behavior.

- The durable command catalog is **42 IDs**: the earlier 41-ID catalog plus WO-002's `ui.status.set`. That command carries an optional, bounded single-line diagnostic message (maximum 512 UTF-8 bytes); the desktop dispatcher is its sole state writer, while the headless executor rejects the desktop-only effect without session mutation. No dependency or lockfile change was reported.
- Native title begins as **Untitled Screenplay** and follows the active FDX filename, including extension, only after successful New/Open/Save/Save As state changes. Failed/cancelled operations retain the previous title. Native decorations and title positioning remain OS-owned; cross-platform centering, especially on Windows, is not guaranteed.
- The status center is exclusively the automatic **Ready / cursor / modifier-guidance** projection. Routine success/development messages do not appear there. Errors and warnings are assigned to Diagnostics; Diagnostics is intentionally disabled by default, so those assigned errors are hidden while disabled, with no row, placeholder, focus target, or accessibility node and no fallback into the center. This is an intentional visibility tradeoff, not a claim that errors remain visible.
- Paragraph guidance consumes the shared core `EditingSession::paragraph_key_targets()` decisions only for an eligible collapsed caret at canonical UTF-8 element end. The projection does not mutate the session or promise that later transactional capacity/preservation checks will accept an edit. Exact held-element choices derive from the existing binding table. macOS uses Command (base) and Command+Control (extended); Windows uses Control (base) and Control+Shift (extended). Focus loss, IME, modal state, unsupported extra modifiers, selection, and interior caret behavior follow the initiative contract; long lists remain one-row clipped and retain full accessible text.
- Runtime menu policy defaults to native Application/catalog menus only on macOS, the in-window catalog only on Windows, and no menu backend on other targets. The source-only dual-preview switch is removed. The process-transient `--menu-platform=auto|macos|windows` argument is parsed before launch; invalid/duplicate/unexpected arguments and unsupported native-macOS requests fail before eframe launch (argument/policy errors report exit code 2). Native menu installation failure aborts startup with contextual error rather than silently selecting no menu or a second accelerator owner; how the launcher surfaces that failure is unverified.
- A macOS `--menu-platform=windows` run is a Windows-style **menu presentation preview**, not OS emulation: menu annotations display Ctrl, while actual macOS host menu accelerators remain Command-based. Editor shortcuts and status labels, native dialogs, clipboard, notification, window controls, page/document-point conversion, and native scale stay host-owned.

The earlier 41-ID count, persistent center operation/error text, always-visible errors, mandatory macOS dual-preview, source-only preview switch, and deferred production OS switching are not current requirements. Earlier dated records remain historical evidence of their then-current implementation, not present behavior or acceptance evidence. Older 2026-10-29 work-order quotations of the former always-visible-error criterion are historical snapshots and remain unedited; the current writing-loop requirement supersedes them. Current requirements supersede earlier records where they conflict.

## Exact WO-007 changed files

Only these documentation/plan files are changed by WO-007:

1. `.forge/requirements/product-overview.md` — current 42-command inventory and approved desktop title/status/menu behavior.
2. `.forge/requirements/features/screenplay-writing-loop.md` — correction of the stale center-text clause and explicit current diagnostics/menu contracts.
3. `.forge/blueprints/technical-context.md` — consolidated title, status, command, runtime menu/accelerator, host-preview boundaries, and submission state.
4. `.forge/project-manifest.md` — WO-007 evidence, exact changed paths, reported branch, preserved-work boundary, and explicit no-acceptance/no-validation status.
5. `.plan/2026-11-06-ui-indicators-platform-shell/ui-indicators-platform-shell-v1.md` — initiative status, current outcomes, exact handoff and user-owned validation links.
6. This work-order handoff.
7. [WO-007 user-validation handoff](WO-007-user-validation-handoff.md) — consolidated user-owned command and platform/regression matrix.

These changes were authored as direct documentation reconciliation from the existing initiative and WO-001–WO-006 handoffs. No production source, test, manifest dependency, lockfile, or other application feature was changed. No branch switch, stash, reset, overwrite, commit, merge, test, build, check, lint, formatting command, launch, manual verification, review, or validation was performed for WO-007.

## Submission and evidence boundary

WO-001–WO-006 are **implementation submissions only**. Their authored tests are reported as authored but unrun. All six remain unaccepted; no implementation is treated as compiled, operationally verified, visually accepted, or validated by this documentation work. The initial branch/worktree facts are reproduced only as reported in the earlier handoffs; WO-007 did not re-inspect or change Git state. All pre-existing tracked and untracked work remains to be preserved.

Known validation gaps/risks recorded by the handoffs:

- WO-002 added no concrete Serde codec round-trip test because the command crate does not select a codec. Deserialization applies the message validator, but serialized wire round-trip behavior remains without concrete coverage. The user should resolve whether host-codec round-trip evidence is sufficient or whether a separately authorized implementation/test change is needed before acceptance; no result is assumed here.
- No compilation or runtime/platform behavior is established for any of WO-001–WO-006. Native title lifecycle and placement, menu installation and launcher failure presentation, keyboard delivery/accelerator uniqueness, diagnostics-enabled rendering, and Windows behavior all require user evidence.
- WO-004 reports that a bounded minimum font size does not prevent the complete modifier list from clipping visually in a narrow center allocation; the accessible label is intended to retain the complete list. Usability remains for the user to inspect.
- The macOS Windows-style menu preview deliberately shows Windows Ctrl annotations while host activation remains Command-based. This mismatch is expected; it must not be mistaken for Windows shortcut simulation.
- The user owns all review and validation. These risks are pending evidence, not agent findings or acceptance.

The consolidated procedure, scenario matrix, and pending-evidence record are in the [WO-007 user-validation handoff](WO-007-user-validation-handoff.md). No validation outcome or acceptance is recorded here.

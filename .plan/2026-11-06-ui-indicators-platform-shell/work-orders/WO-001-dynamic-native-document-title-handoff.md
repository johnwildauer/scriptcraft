# WO-001 — Dynamic native document title handoff

- **Initiative:** [UI indicators and platform shell v1](../ui-indicators-platform-shell-v1.md)
- **Work order:** Dynamic native document title only
- **Submission date:** 2026-11-07
- **Branch:** `dev/2026-11-06-ui-indicators-platform-shell`
- **HEAD:** `20496cdde93a22bbfccef62b009a36c44f712b67` (same as starting checkout)
- **Validation state:** Not run; user-owned validation and acceptance pending

## Checkout and branch evidence

Before branch transition, non-destructive Git inspection observed:

- Current branch: `master`
- HEAD: `20496cdde93a22bbfccef62b009a36c44f712b67` (`Merge hotkey command layer`)
- Tracked worktree modifications: none
- Staged changes: none
- Untracked work: `.plan/2026-11-06-ui-indicators-platform-shell/` (the user-provided assessment)
- Destination local branch: absent

Created and switched to `dev/2026-11-06-ui-indicators-platform-shell` from that checkout using the
ordinary branch-creation/switch operation. The existing untracked initiative directory was preserved.
No reset, stash, overwrite, commit, merge, or destructive action was performed. Final branch and
working-tree state after authoring the submission was:

- Branch and HEAD remain `dev/2026-11-06-ui-indicators-platform-shell` and
  `20496cdde93a22bbfccef62b009a36c44f712b67`.
- Staged paths: none.
- Modified tracked, unstaged paths: [`.forge/blueprints/technical-context.md`](../../../.forge/blueprints/technical-context.md),
  [`.forge/project-manifest.md`](../../../.forge/project-manifest.md),
  [`.forge/requirements/features/screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md),
  [`apps/scriptcraft/src/app_commands.rs`](../../../apps/scriptcraft/src/app_commands.rs), and
  [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs).
- Untracked directory: `.plan/2026-11-06-ui-indicators-platform-shell/`, containing exactly
  [`ui-indicators-platform-shell-v1.md`](../ui-indicators-platform-shell-v1.md) and this handoff at
  the time of the inventory.
- No other tracked or untracked work was present in the final status output.

## WO-001 submission

Implemented only the native document title behavior:

- Added a pure projection from the active path's final UTF-8 filename component, preserving its
  extension, with `Untitled Screenplay` for no path or unusable/empty basename.
- Configured the initial native viewport and eframe window title as `Untitled Screenplay`.
- Cached the applied title and synchronize it through the existing `DesktopViewportEffects` boundary
  only when the projection changes. Synchronization occurs at the end of the desktop frame, after
  dispatched state transitions.
- Existing native path changes occur only on successful New, Open activation, or transactional Save /
  Save As completion. Failed and cancelled paths leave the active native path unchanged, so the
  projected title remains unchanged. Save without changing the filename, repeated frames, and moving
  the same filename between directories do not send redundant title effects.
- Kept native decorations and OS-owned title positioning. No custom title bar, dirty suffix, full
  path, duplicate app-name suffix, FDX field, dependency, or persistence behavior was added.

## Changed files

- [`apps/scriptcraft/src/main.rs`](../../../apps/scriptcraft/src/main.rs) — title projection/cache, frame synchronization, startup title, and authored tests.
- [`apps/scriptcraft/src/app_commands.rs`](../../../apps/scriptcraft/src/app_commands.rs) — native title viewport effect.
- [`.forge/requirements/features/screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md) — native-title lifecycle contract.
- [`.forge/blueprints/technical-context.md`](../../../.forge/blueprints/technical-context.md) — projection/effect ownership and transient-state boundary.
- [`.forge/project-manifest.md`](../../../.forge/project-manifest.md) — exact implementation submission and policy-compliant validation status.
- [`ui-indicators-platform-shell-v1.md`](../ui-indicators-platform-shell-v1.md) — WO-001-only initiative status and handoff reference.
- This handoff.

The whole initiative directory was untracked before this task; therefore all files beneath it,
including the original user-provided assessment and this handoff, remain untracked repository content.
No user-owned or earlier staged work was modified.

## Authored coverage

Six focused unit tests were authored in `main.rs` for:

1. Unsaved/new title fallback and reset projection.
2. Basename-only projection with spaces and extension.
3. Open and Save As filename projection after successful activation/save.
4. Unicode filename and extension preservation.
5. Deduplication for equal projected titles and changed-title projection.
6. Unchanged active-path
   projection corresponding to failed/cancelled operations that preserve state.

These tests cover the pure projection/update seam, not GUI/native-window integration or actual file
picker and storage transactions. No tests were run. No build, check, lint, formatter, launch, manual
verification, agent review, or validation was run, as required by repository policy and the task.

## User-owned acceptance still pending

- Confirm startup and successful New display `Untitled Screenplay`.
- Confirm successful Open, ordinary Save, first Save As, and subsequent Save As display the active
  filename with extension, no directory path, and no dirty marker or duplicate application suffix.
- Confirm failed/cancelled Open, Save, and Save As preserve the prior native title.
- Review long, spaced, and Unicode filenames and native title placement on macOS and Windows.
- Confirm native decorations, OS-owned centering/placement, and unchanged content/toolbar layout.
- User performs any desired compilation, test execution, runtime review, and final acceptance.

WO-002 through WO-007 remain pending and were not modified or implemented by this handoff.

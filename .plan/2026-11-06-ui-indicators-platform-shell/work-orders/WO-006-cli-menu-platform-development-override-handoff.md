# WO-006 — CLI menu-platform development override handoff

- **Submission date:** 2026-11-07
- **Branch:** `dev/2026-11-06-ui-indicators-platform-shell` (existing user-designated branch; no switch)
- **State:** Implementation submitted; user review, validation, and acceptance pending
- **Scope:** WO-006 only. WO-001–WO-005 were preserved; WO-007 was not implemented.
- **Validation owner:** User under the authoritative policy in [project-manifest.md](../../../.forge/project-manifest.md).

## Result and usage

Desktop startup accepts one process-transient `--menu-platform=auto|macos|windows` argument. The
approved development invocation is:

```text
cargo run -p scriptcraft -- --menu-platform=windows
```

Omitting the argument selects `auto`. Explicit `auto`, `macos`, and `windows` values use the existing
WO-005 `RuntimeMenuPolicy::resolve()` policy against the actual compile target. No menu selection is
persisted, exposed in settings, or used to impersonate a different host OS.

| Compile target | Choice | Selected menu behavior |
|---|---|---|
| macOS | `auto` / `macos` | Native Application and catalog menus only; native accelerator owner |
| macOS | `windows` | In-window egui strip only; no native controller construction/install; in-window accelerator owner |
| Windows | `auto` / `windows` | In-window egui strip only; in-window accelerator owner |
| Windows | `macos` | Rejected before eframe launch |
| Other | `auto` | No menu backend |
| Other | `windows` | In-window egui strip only; in-window accelerator owner |
| Other | `macos` | Rejected before eframe launch |

Argument parsing and host-policy resolution run before eframe launch. Empty/unknown values, duplicate
options, a missing equals-form value, unknown options, positional arguments, and non-UTF-8 arguments
are rejected with a concise usage message and nonzero exit (argument/policy errors use exit code 2).
There is deliberately no space-separated form; use `--menu-platform=<value>`. No argument-parser
dependency was added.

## Host behavior and simulation limitation

The macOS `windows` option simulates the Windows-style menu presentation only. It is **not full OS
emulation**. Menu annotations remain Windows-style catalog presentation, but actual shortcut matching
continues to use the host: on macOS primary menu accelerators remain Command-based, not Control-based.
Editor shortcut behavior and status labels remain host-accurate (including macOS Command labels).
Document-point conversion, native display scaling, native file dialogs, clipboard, notifications,
window controls, compiled platform dependencies, screenplay/document behavior, and other host
integrations are not switched. The selected runtime policy remains the sole gate for native install,
in-window rendering, native event drain/synchronization, and raw-input menu shortcut extraction, so
only one actual host accelerator owner is selected at a time.

## Deterministic failures

- No argument: `Auto` (not an error).
- `--menu-platform`: missing equals-form value; rejected.
- `--menu-platform=` or an unsupported/case-mismatched value: rejected with allowed values.
- Repeated `--menu-platform` option, including a repeated bare form: rejected as duplicate when
  encountered after a prior valid option.
- Unknown switches or positional values: rejected rather than ignored.
- Non-UTF-8 argument: rejected without lossy reinterpretation.
- `--menu-platform=macos` on Windows/other: rejected before constructing presentation/native menu or
  launching eframe; Auto and the supported Windows-style in-window request retain their target-specific
  behavior.

## Exact changed paths

- `apps/scriptcraft/src/main.rs` — standard-library argument parser, deterministic usage/policy errors
  before launch, actual-target startup policy resolution, and parser/policy/ownership tests.
- `.forge/requirements/features/screenplay-writing-loop.md` — process-transient CLI contract,
  validation failures, one-owner behavior, host-integration boundary, and simulation limitation.
- `.forge/blueprints/technical-context.md` — startup parsing/resolution, host-accurate accelerator
  behavior, and explicit separation of visual simulation from OS emulation.
- `.forge/project-manifest.md` — WO-006 implementation submission, paths, no-validation evidence, and
  branch-preservation statement.
- `.plan/2026-11-06-ui-indicators-platform-shell/ui-indicators-platform-shell-v1.md` — current
  initiative status and WO-006 implementation submission only.
- This handoff.

No menu catalog, `menu.rs` policy implementation, shortcut table, native adapter, app/package manifest,
dependency, lockfile, document, scale, or platform integration source was changed. Existing WO-005
runtime policy gates already provide the exclusive backend/accelerator ownership; this work supplies
the startup argument adapter.

## Authored coverage; not executed

Focused tests in `main.rs` cover:

- Default `Auto` and every accepted value.
- Missing equals-form value, empty and case-mismatched/unknown value, duplicate option, unknown switch,
  unexpected positional argument, and (on Unix) non-UTF-8 input; checks deterministic usage-bearing
  errors.
- Startup resolution on macOS, Windows, and other targets, including macOS Windows-style override,
  unsupported native-macOS choices, exclusive renderer selection, and one accelerator owner.

Existing focused policy tests in `menu.rs` cover Auto defaults, explicit in-window policy, unsupported
native requests, renderer choice, and accelerator-owner projections. These are policy/parser assertions,
not evidence of process output, target compilation, native installation behavior, actual OS keyboard
delivery, rendering/accessibility, or no-duplicate runtime command activation.

No test, build, check, lint, formatting command, launch, manual verification, review, independent
validator, or other validation was run. Compilation and all runtime behavior remain unverified. User
review, validation, and acceptance are pending. No implementation blocker or dependency-parser need
was identified from the inspected source.

## User-owned acceptance focus

1. Run Auto on macOS and Windows and confirm native-only macOS versus in-window-only Windows behavior.
2. Run `cargo run -p scriptcraft -- --menu-platform=windows` on macOS; confirm no native controller,
   exactly one in-window strip/accelerator owner, and no duplicate menu action.
3. Confirm macOS host Command behavior and editor/status labels remain unchanged despite Windows-style
   menu annotations; review this documented modifier mismatch in the preview.
4. Confirm Windows `--menu-platform=macos` and non-macOS `macos` requests fail before window/menu
   launch, and exercise malformed, duplicate, unknown, positional, and non-UTF-8 cases.
5. Confirm other-target Auto has no menu and explicit Windows selection uses the in-window backend;
   verify file dialogs, clipboard, notification, scale, window controls, and document behavior remain
   host-specific.
6. Perform any desired user-owned tests/build/checks, launches, and review. This handoff records no
   validation outcome or acceptance.

WO-007 remains pending and was not included.

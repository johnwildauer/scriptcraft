# WO-005 — Runtime OS-default menu policy handoff

- **Submission date:** 2026-11-07
- **Branch:** `dev/2026-11-06-ui-indicators-platform-shell` (existing user-designated branch; no switch)
- **State:** Implementation submitted; user review, validation, and acceptance pending
- **Scope:** WO-005 only. WO-001–WO-004 remain preserved; WO-006–WO-007 were not implemented.
- **Validation owner:** User under the authoritative policy in [project-manifest.md](../../../.forge/project-manifest.md).

## Result

Replaced the source-only macOS dual-preview switch and compile-target helper functions with one
app-owned runtime policy. The production startup path resolves `Auto` once and stores the resolved
policy in `ScriptCraftApp`.

| Compile target | `Auto` backend | Menu accelerator owner |
|---|---|---|
| macOS | Native Application menu plus all ten shared native catalog menus only | Native macOS |
| Windows | Complete in-window catalog only | In-window |
| Other | None | None |

An explicit in-window choice is accepted on any target for development. A native macOS choice is
rejected off macOS, before any native controller can be constructed. Native modules and adapters
remain cfg-gated to macOS. No CLI parser, settings UI, persisted preference, or host-platform spoofing
was added.

Native installation, in-window panel composition, native event drain and availability synchronization,
and raw-input shortcut extraction all consult the same app-owned selection. The in-window renderer and
native controller cannot both be active under this policy, and exactly the selected menu backend owns
menu accelerators. In-window shortcut recognition continues to use actual host shortcut semantics;
selecting a presentation does not change editor shortcut labels/routing, document-point conversion,
native scale, file dialogs, clipboard, notification, or other host behavior. Shared catalog entries,
placeholders, FIFO ordering, dispatcher/availability checks, focus restoration, status guidance,
native title, and document state were not intentionally changed.

If native macOS menu installation fails, the app-creation result now includes contextual failure text
and aborts startup. It does not silently continue with no selected menu or start a competing in-window
accelerator owner. Actual error surfacing by eframe/launcher remains unverified.

## Policy API for WO-006

In [menu.rs](../../../apps/scriptcraft/src/menu.rs), the injectable request is
`MenuPlatformChoice::{Auto, MacOS, Windows}`. `MenuHostPlatform::{MacOS, Windows, Other}` represents
the actual compile host for pure policy resolution. `RuntimeMenuPolicy::resolve(host, requested)`
returns `Result<RuntimeMenuPolicy, UnsupportedMenuPlatformChoice>` and is deterministic and UI-free.
Production presently uses `RuntimeMenuPolicy::for_current_target()`; `main.rs` carries this value in
`StartupPresentation`, then passes it to `ScriptCraftApp::new_with_menu_policy()` before native
installation. The `RuntimeMenuPolicy` value exposes backend/renderer/installation/accelerator-owner
projections so all runtime routes use the same result.

WO-006 may parse a command-line string into `MenuPlatformChoice`, call `resolve()` before eframe/native
menu creation, and pass the resulting policy into app creation. It should preserve the reported host
for editor and platform behaviors, report unsupported/duplicate/malformed arguments before launch,
and avoid changing the backend API unless a concrete need is found. No argument parsing behavior is
implemented here.

## Changed paths

- [menu.rs](../../../apps/scriptcraft/src/menu.rs) — runtime request/host/backend types, pure resolver,
  single-owner projections, removal of preview/compile-only policy, and source-level policy tests.
- [main.rs](../../../apps/scriptcraft/src/main.rs) — app-owned resolved policy and injection seam;
  unified policy gates for native install/drain/sync, egui panel rendering, and raw shortcut ownership;
  contextual native-installation startup failure.
- [screenplay-writing-loop.md](../../../.forge/requirements/features/screenplay-writing-loop.md) —
  native-only macOS and in-window-only Windows defaults, other-target fallback, one backend/owner,
  startup failure, and explicit supersession of the old dual-preview instructions.
- [technical-context.md](../../../.forge/blueprints/technical-context.md) — current resolver,
  ownership flow, startup behavior, and explicit supersession of the preview deferral.
- [project-manifest.md](../../../.forge/project-manifest.md) — submission-only changed-path and
  validation evidence.
- [ui-indicators-platform-shell-v1.md](../ui-indicators-platform-shell-v1.md) — WO-005 implementation
  submission and current WO-006/WO-007 state.
- This handoff.

No Cargo manifest, dependency, lockfile, native adapter implementation, menu catalog, command,
shortcut catalog, toolbar, status, core, document, or persistence file was changed by WO-005.

## Authored source-level coverage; not executed

Focused tests in `menu.rs` cover:

- `Auto` resolution to native macOS, in-window Windows, and no backend on other targets.
- Explicit in-window selection and the unique matching accelerator owner, including macOS in-window
  override and the other-target development fallback.
- Renderer, native-installation, and in-window-shortcut-owner projections for each selected backend.
- Native macOS request acceptance on macOS and rejection on Windows/other targets.

The tests assert policy decisions only; they do not prove native module compilation, controller
installation, app-creation error presentation, real OS event delivery, actual keyboard behavior,
rendering/accessibility, menu activation ordering, or native focus behavior.

No test, build, check, lint, formatting command, launch, manual verification, review, independent
validator, or other validation was run. Compilation and all runtime/platform behavior remain
unverified. User review, validation, and acceptance are pending. There were no source-work blockers;
the startup-failure visibility and platform behavior are specifically retained as user-owned risks.

## User-owned acceptance focus

1. Confirm ordinary macOS presents only native Application/catalog menus and no in-window strip; check
   native item labels, availability, disabled placeholders, shortcuts, and event dispatch.
2. Confirm Windows presents the full in-window catalog and no native menu construction; exercise
   menu click, popup switching, keyboard navigation/dismissal, shortcut activation, and editor focus
   restoration.
3. Confirm non-macOS native requests reject before launch/native construction and the other-target
   Auto fallback presents neither menu renderer.
4. If WO-006 adds the Windows-style macOS override, confirm only the in-window renderer/accelerator
   owner activates, while actual macOS Command shortcuts and all other host integrations remain
   host-correct.
5. Review native-install failure reporting, command FIFO/availability behavior, toolbar boundary,
   status/title/document preservation, and absence of duplicate activation.
6. User performs any desired compilation, tests, runtime review, and acceptance under repository
   policy.

This handoff is not validation evidence or acceptance. WO-006 and WO-007 remain pending.

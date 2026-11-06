# WO-010 — Durable menu contracts and development closeout

- **Status:** Implementation submitted; user review and validation pending
- **Initiative:** [`menu-system-v1.md`](../menu-system-v1.md)
- **Depends on:** [`WO-009`](WO-009-native-events-and-dual-preview-arbitration.md)
- **Validation owner:** User; no agent-run checks, reviews, or validators

## Single outcome

Reconcile the implemented menu system with durable project records and submit the complete dual-presentation development state for user acceptance without disabling the in-window preview.

## Scope

1. Confirm in source that the in-window preview switch in [`menu.rs`](../../../apps/scriptcraft/src/menu.rs) remains true. Defer automatic production OS-only selection and switch-off to a later explicitly approved change.
2. Add REQ-WRITING-LOOP-011 in [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md), covering shared catalog/dispatcher ownership, exact initial items, native macOS and in-window Windows presentations, current macOS dual preview, accelerator arbitration, canonical clipboard behavior, and transient desktop state.
3. Update [`technical-context.md`](../../../.forge/blueprints/technical-context.md) with the module boundaries, menu/toolbar/Script Workspace ordering, command queue/focus behavior, clipboard adapter, native event flow, preview policy, and absence of submenu/core/persistence changes.
4. Update [`project-manifest.md`](../../../.forge/project-manifest.md) with actual dependencies and implementation-submission facts only. Preserve historical evidence and repository-wide user-owned validation policy.
5. Reconcile every work-order handoff and [`menu-system-v1.md`](../menu-system-v1.md) with the actual implementation, including initial metrics, dependency versions, known limitations, and the pre-existing New consent discrepancy.
6. Do not add an automated desktop-test target or mark any prior order validated. Do not claim Windows acceptance from the macOS preview.

## User-owned final review

- Run desired workspace formatting, check, build, Clippy, and consolidated core acceptance commands; record exact outcomes separately.
- On macOS, verify native Application/File/Edit menus and the in-window File/Edit preview are simultaneously visible and functional, with native-only accelerator ownership.
- On Windows, verify the egui strip appears directly above the toolbar and all Ctrl shortcuts and commands work without any macOS backend.
- Repeat visual, clipboard, Unicode selection, focus, modal, Appearance, resize, short-viewport, and display-scale checks on both targets.
- Record final acceptance or required remediation explicitly; implementation submission alone is not completion.

## Handoff requirements

The final handoff must list changed durable documents, all user evidence still pending, the enabled preview-switch value, and the deferred automatic-production decision. No agent-run validation or independent review is permitted under current policy.

## WO-010 implementation handoff — 2026-11-06

- **State:** Implementation submitted for user review/validation only; not accepted or validated. Work remained on the existing `dev/2026-11-06-menu-system` branch. The preview switch is confirmed `true` and remains enabled.
- **Changed durable records:** Added REQ-WRITING-LOOP-011 in `screenplay-writing-loop.md`; updated `technical-context.md` with actual module ownership, panel order, queue/focus/clipboard/native event flow, dual-preview policy, source metrics, and non-goals; updated `project-manifest.md` with actual dependency and implementation-submission facts while preserving historical evidence and user-owned validation policy; reconciled the initiative plan and WO-001–WO-009 handoffs with integrated source.
- **Dependency and source facts:** Direct desktop clipboard dependency `arboard = "3.7.0"`; macOS dependency declaration `muda = { version = "0.17", default-features = false }`, lockfile resolution 0.17.2. With a registered `muda` 0.17.2 event handler, native IDs are delivered through the handler-fed inbox, not `MenuEvent::receiver()`. Native macOS owns accelerators while installed, including dual preview; Windows uses in-window accelerator ownership.
- **Evidence status:** No builds, checks, tests, lint, formatting, launch, manual verification, review, or independent validation were run. No automated desktop tests were added. User validation and acceptance remain pending; the initiative handoff lists the required platform, command, clipboard, focus/modal, visual/scale, and optional workspace evidence.
- **Deferred and known boundaries:** No automatic production OS-only switch, preview disablement, submenu, or New-consent repair was implemented. The pre-existing New dirty-session consent discrepancy remains. The source-level implementation submission is not validation or acceptance.

### User-reported WO-009 E0282 source correction — 2026-11-06

The user reported E0282 at `MenuEvent::set_event_handler` in [`macos_menu.rs`](../../../apps/scriptcraft/src/macos_menu.rs), where the untyped callback parameter left `event.id`'s type unknown. The exact correction is `move |event: MenuEvent|`. This only annotates the existing native `muda 0.17.2` callback; eframe remains 0.34 (locked 0.34.3), and no dependencies were changed. No build or other validation was run, and no successful compilation is claimed.

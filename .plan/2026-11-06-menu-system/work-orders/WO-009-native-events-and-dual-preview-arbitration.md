# WO-009 — Native macOS events, state sync, and dual-preview arbitration

- **Status:** Implementation submitted for user review; not accepted or validated
- **Initiative:** [`menu-system-v1.md`](../menu-system-v1.md)
- **Depends on:** [`WO-008`](WO-008-native-macos-menu-construction.md)
- **Validation owner:** User; no agent-run checks, reviews, or validators

## Single outcome

Make native macOS items fully activate shared commands, stay synchronized with application state, and coexist with the clickable in-window preview without duplicate shortcuts.

## Scope

1. Map retained native menu IDs back to shared command identities in [`macos_menu.rs`](../../../apps/scriptcraft/src/macos_menu.rs).
2. Drain the documented native menu-event receiver into the pending application command queue. Install the event handler needed to request an egui repaint through a cloned context so idle application state wakes for menu activation.
3. Recheck current command availability when dispatching native events, rejecting stale activation during migration-modal or incompatible state transitions.
4. Synchronize native item labels and enabled states from the same availability projection used by egui after history, selection, document, save, and modal changes.
5. Make native macOS the sole accelerator owner whenever its backend is active, including dual preview. Keep both native and in-window items clickable.
6. Update the raw-input shortcut policy from WO-007 so native-owned shortcuts are not also enqueued by egui. Windows remains in-window owned.
7. Preserve custom Cut/Copy/Paste command routing; do not substitute AppKit responder actions. Keep native appearance and accessibility OS-owned.

## WO-009 implementation handoff — 2026-11-06

- **State and scope:** WO-009 implementation is submitted for user review and validation only; it is not accepted or validated. Work remained on the existing `dev/2026-11-06-menu-system` branch, preserving prior work. No branch or commit was created. The in-window preview switch remains `true`; WO-010 has not started.
- **Native event and wakeup path:** [`macos_menu.rs`](../../../apps/scriptcraft/src/macos_menu.rs) retains a `MenuId` → `(AppCommand, native item handle)` map for every catalog command. The `muda 0.17.2` `MenuEvent::set_event_handler` callback captures a cloned egui context and a thread-safe ID inbox; each callback appends the activated native ID and calls `Context::request_repaint()`. At the beginning of the next application update, [`main.rs`](../../../apps/scriptcraft/src/main.rs) drains that inbox, resolves retained IDs to shared commands, and appends them to the existing FIFO before its ordinary pending-command dispatch. Unknown IDs are ignored.
- **Resolved API constraint:** In `muda 0.17.2`, registering a non-`None` event handler suppresses delivery to `MenuEvent::receiver()`; the handler and documented receiver are mutually exclusive. To satisfy the required event-loop wakeup without losing activation IDs, this implementation drains the handler-fed ID inbox into the same pending command queue rather than polling `MenuEvent::receiver()`. This is an API-driven integration detail, not a runtime verification claim.
- **Availability and synchronization:** [`AppCommandAvailability`](../../../apps/scriptcraft/src/app_commands.rs) is now the shared enabled-state projection for egui and native rows. It includes modal blocking, and `dispatch_app_command()` rechecks it for every activation, rejecting stale commands and all menu commands while the FDX migration modal is pending. Native labels are refreshed from the catalog and enabled states from that projection at the end of every application update, after queued dispatch, editor/state processing, and panel/dialog handling. egui rows use the same projection. Native File/Edit items remain custom ScriptCraft commands, including Cut/Copy/Paste; native OS presentation, shortcuts, and accessibility remain muda/AppKit-owned.
- **Accelerator ownership:** Raw-input interception now grants in-window accelerator ownership only when native macOS is not active. Thus macOS native accelerators are sole owner even with the clickable egui preview visible; matched events are removed before editor handling, avoiding duplicate queueing. Windows remains in-window owned. The preview remains enabled and no production switch-off was introduced.
- **Changed paths:** `apps/scriptcraft/src/app_commands.rs`, `apps/scriptcraft/src/macos_menu.rs`, `apps/scriptcraft/src/main.rs`, `apps/scriptcraft/src/menu.rs`, this work order, and the initiative plan.
- **Evidence limitations and follow-up:** No build, check, test, lint, formatting, launch, manual verification, review, or independent validator was run, as required by repository policy. The wakeup, native activation, receiver/handler integration, availability synchronization, accelerator arbitration, clipboard routing, and OS appearance/accessibility remain unverified at runtime and require user-owned validation. This handoff makes no runtime, acceptance, or validation claim.

### User-reported E0282 compile-fix handoff — 2026-11-06

- The user reported compiler error **E0282** at `MenuEvent::set_event_handler` in [`macos_menu.rs`](../../../apps/scriptcraft/src/macos_menu.rs): the closure parameter type was not inferred where `event.id` is accessed.
- The narrow source fix explicitly annotates the callback parameter as `MenuEvent`: `move |event: MenuEvent|`. The locally imported type is from native dependency `muda 0.17.2`.
- No eframe/egui version or dependency was changed; eframe remains 0.34 with lockfile resolution 0.34.3, and this report is not an egui error. No other design or dependency change was made.
- No build or other validation was run. This records the user-supplied compiler evidence and exact source correction only; compilation is not claimed to pass and remains for user-owned validation.

### WO-010 integrated-source reconciliation

The actual final path is handler callback → thread-safe native-ID inbox plus repaint request → inbox
drain in `ScriptCraftApp::update()` → known-ID lookup → shared `PendingMenuCommands` FIFO → ordered
dispatcher with fresh availability check. Native labels/enabled states synchronize after panel and
state processing. The resolved `muda` 0.17.2 handler/receiver exclusivity is reflected above. The
preview remains enabled and native macOS is the sole accelerator owner when installed. None of these
integration paths has been runtime-verified or accepted.

## Structural acceptance for user review

- Native pointer and accelerator activation enter the same queue/dispatcher as every other source.
- A native event wakes an idle app and executes once; in-window shortcut handling cannot duplicate it.
- Native and egui enabled states agree after each relevant state transition.
- The preview switch remains on and no production OS-only switch-off occurs.

## User-owned behavior review

- Activate every native item by pointer and Command shortcut, then repeat through the in-window preview and compare results.
- Verify idle wakeup, Undo/Redo and Cut/Copy enablement, selection changes, Save/New/Open, document replacement, and stale modal events.
- Confirm no double edit/file action in dual preview and correct Ctrl behavior on Windows.
- Verify custom clipboard commands operate on the screenplay session, not a native responder target.

## Handoff requirements

Record native event/wakeup implementation, synchronization cadence, final accelerator ownership, and any OS-specific limitations requiring follow-up. No runtime claim is made before user validation.

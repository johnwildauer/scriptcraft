# Stage 2 — Command/session architecture and the UI boundary

## Scope and method

This report is a source-based analysis of the reusable editing boundary in WordCraft for a small
screenplay-writing application. It follows one edit from an input event to command dispatch,
document mutation, undo state, layout invalidation and automation, then separates repository facts
from recommendations. The relevant implementation is intentionally read as an API boundary rather
than as a proposal to copy WordCraft's full feature catalog.

The main sources are [`Session::run()`](../../crates/engine/src/session.rs:1116), the
[`CommandSpec`](../../crates/engine/src/lib.rs:58) and [`Registry`](../../crates/engine/src/lib.rs:102)
definitions, the egui input bridge in [`keys::canvas_events()`](../../crates/ui-egui/src/keys.rs:199),
and the automation paths in [`control::handle()`](../../crates/ui-egui/src/control.rs:107) and
[`mcp::Headless::answer()`](../../crates/mcp/src/lib.rs:38).

## Executive assessment

WordCraft has a strong reusable core for a 20–40-command screenplay editor:

* a document/session object that owns selection, view-independent editing state, dirty state,
  history and layout cache;
* a small, stable command protocol (`id + JSON params -> JSON result/error`) used by UI, CLI,
  control and MCP rather than separate business logic for each frontend;
* centralized transactional behavior in [`Session::run()`](../../crates/engine/src/session.rs:1116),
  including enablement checks, undo checkpointing, rollback on command errors, selection clamping,
  revision tracking and an escaped-panic safety net;
* a keymap that resolves physical/editor shortcuts to command IDs, leaving key interpretation in the
  UI but editing semantics in the engine; and
* headless execution and rendering that make most behavior testable without a window.

The screenplay application should retain this *shape*, not the Word-specific breadth. It should
reuse the command/session ideas and perhaps the existing lower-level document/layout crates, while
keeping screenplay UI state, dialogs and host services outside the engine. A small product should
avoid importing the full ribbon/catalog architecture: a focused command registry with roughly
30 commands, a compact menu/toolbar/palette, and a canvas/editor adapter is sufficient.

## 1. Observed facts

### 1.1 The engine boundary

The engine explicitly states that every user-visible action is a command and that menus, ribbon,
shortcuts, command search, CLI, control and MCP all call [`Session::run()`](../../crates/engine/src/session.rs:1116)
([`engine/src/lib.rs`](../../crates/engine/src/lib.rs:1)). The public command result is
[`CmdResult`](../../crates/engine/src/lib.rs:56), a `Result<Value, CmdError>`, with typed error
categories for unknown commands, bad parameters, disabled commands and execution failure
([`CmdError`](../../crates/engine/src/lib.rs:38)). This is a useful automation contract: callers do
not need to understand Rust types to invoke a command, but errors remain distinguishable inside
the engine and become strings at transport boundaries.

Each [`CommandSpec`](../../crates/engine/src/lib.rs:58) contains a stable ID, human label, UI
location, portable shortcut, parameter documentation, a `mutates` flag, an `enabled` predicate and
a function pointer. The constructor defaults a command to mutating and enabled; [`pure()`](../../crates/engine/src/lib.rs:91)
marks navigation, view and export-like commands non-mutating, while [`when()`](../../crates/engine/src/lib.rs:96)
adds state-dependent availability. The registry is a vector plus ID hash map, supports lookup,
description and shortcut lookup, and serializes command metadata for agents
([`Registry`](../../crates/engine/src/lib.rs:102)).

Command implementations are grouped by domain and assembled in one factory
([`cmd::registry()`](../../crates/engine/src/cmd/mod.rs:57)). WordCraft's current registry is much
larger than the screenplay target; the architecture does not require that scale. A screenplay
registry can expose only writing, screenplay element, navigation, formatting, search, save/export,
undo/redo and project metadata commands.

### 1.2 What `Session` owns

[`Session::new()`](../../crates/engine/src/session.rs:563) initializes a document, caret selection,
view state, preferences, command registry, keymap, ribbon state, dirty flag, revision and layout
cache. The important reusable distinction is that the session is not just the document: it is the
editing transaction context. It also contains host-facing state such as path, clipboard, status,
author and UI requests; a screenplay product should decide which of those belong in a core session
and which should be injected or moved to an application shell.

Document revision is bumped by [`Session::touch()`](../../crates/engine/src/session.rs:683), which
also marks the document dirty. [`Session::layout()`](../../crates/engine/src/session.rs:727) caches
the `DocLayout` against the revision plus width, height, view mode, proofing, outline, deletion and
numeral-display inputs. A cache hit returns the previous `Arc`; otherwise layout is recomputed and
stored. [`Session::relayout()`](../../crates/engine/src/session.rs:782) explicitly clears the
layout and layout cache for changes such as font database updates or view settings.

This means a screenplay editor can use a single session-level invalidation rule: document edits
call the centralized mutation path, which changes the revision; view-only changes call relayout
when their layout inputs change. The UI does not need to manually invalidate every page after every
keystroke.

### 1.3 Command execution transaction

The observed execution sequence in [`Session::run()`](../../crates/engine/src/session.rs:1116) is:

1. Consume a pending `join_next` flag, look up the command, and reject unknown or disabled IDs.
2. For mutating commands, run protection checks and establish recording/repeat metadata.
3. Snapshot document, selection, undo/redo stacks, typing-group state, eviction count and column
   selection before running the implementation ([`before_doc`](../../crates/engine/src/session.rs:1156)).
4. Treat `text.insert` and equation typing specially for coalescing; other mutations close the
   typing group. Create a checkpoint unless a drag/update is explicitly joining the previous step
   ([`checkpoint()`](../../crates/engine/src/session.rs:806),
   [`join_next_undo()`](../../crates/engine/src/session.rs:835)).
5. Invoke the command through the column dispatcher inside `catch_unwind`; an escaped panic is
   converted to [`CmdError::Failed`](../../crates/engine/src/session.rs:1192), not allowed to reach
   the UI.
6. On success, record tracked formatting where applicable, call [`touch()`](../../crates/engine/src/session.rs:683),
   ensure a nonempty document, prune stale text boxes, reroute connectors if needed, clamp the
   selection, update edit marks and clear transient secondary selections.
7. On an error, restore the pre-command document, selection, history, redo stack, typing state,
   eviction count and column selection, and put the error in the session status
   ([`Session::run()` rollback](../../crates/engine/src/session.rs:1240)).

The result is a command-level transaction: command implementations can be ordinary mutation code,
while the session owns cross-cutting safety. A screenplay app should retain this arrangement, but
should keep the rollback snapshot narrowly defined and document any command that mutates external
state (filesystem, network, OS clipboard) because in-memory rollback cannot undo those side effects.

### 1.4 Undo/redo behavior

Undo snapshots are cheap because document blocks are shared. [`checkpoint()`](../../crates/engine/src/session.rs:806)
stores the document and selection before the mutation, clears redo, and groups consecutive typing
commands under the `Typing` label. Automatic corrections can be separated with
[`push_undo()`](../../crates/engine/src/session.rs:825). [`undo()`](../../crates/engine/src/session.rs:891)
and [`redo()`](../../crates/engine/src/session.rs:910) swap document/selection snapshots and call
[`touch()`](../../crates/engine/src/session.rs:683), thereby invalidating layout and marking the
document dirty. The implementation caps history at 500 steps
([`MAX_UNDO`](../../crates/engine/src/session.rs:543)).

The typing tests show the intended semantic granularity: consecutive typing is one undo step,
selection movement ends the typing group, typing over a selection can be a separate step, and
AutoFormat/AutoCorrect can be undone independently
([`tests_typing.rs`](../../crates/engine/src/tests_typing.rs:29),
[`typing_then_selecting_then_typing_are_two_undo_steps()`](../../crates/engine/src/tests_typing.rs:496),
[`undo_right_after_autoformat_undoes_only_the_autoformat()`](../../crates/engine/src/tests_typing.rs:167)).
For screenplay writing, this is especially valuable for fast dialogue typing and automatic element
transitions: the product should specify whether an auto-transition is part of the Enter command's
undo step or a distinct correction step.

### 1.5 Keyboard and typing path

The engine's typing commands are ordinary registry entries. [`text::specs()`](../../crates/engine/src/cmd/text.rs:11)
registers `text.insert`, paragraph creation, line/page breaks, Tab, Backspace, Delete and related
editing commands with shortcuts. [`text.insert`](../../crates/engine/src/cmd/text.rs:88) validates
the required text parameter, applies typing/overtype selection rules, runs optional autoformat and
autocorrect, calls the shared text insertion routine, and may push a distinct AutoFormat undo
snapshot.

At the UI boundary, [`keys::canvas_events()`](../../crates/ui-egui/src/keys.rs:199) consumes egui
events. Text and IME commits call `app.run("text.insert", ...)`; Enter/Tab/Backspace and other keys
are resolved through [`dispatch()`](../../crates/ui-egui/src/keys.rs:118), which asks the session
keymap for a command and then calls `app.run`. Paste/copy/cut similarly map to command IDs, with
clipboard adaptation in the UI. [`global_shortcuts()`](../../crates/ui-egui/src/keys.rs:385) repeats
the same dispatch when the canvas is not focused, but only for modifier/function-key shortcuts.

The UI therefore interprets egui/platform events, while the engine interprets editing intent. A
screenplay UI should preserve this split: IME, key repeat, focus, text fields and platform shortcut
normalization belong in the frontend; “new scene heading”, “continue element”, “insert page break”
and “move caret by screenplay unit” belong in commands.

### 1.6 UI boundary and `WordApp`

[`WordApp`](../../crates/ui-egui/src/lib.rs:343) contains the session plus egui-specific state:
canvas geometry and interaction, dialogs, panes, keytips, previews, synthetic input, host service
callbacks, printing, screenshots and control-channel plumbing. [`Services`](../../crates/ui-egui/src/lib.rs:88)
injects platform capabilities such as file dialogs, clipboard, downloads, printing, async file
inbox and dirty-state notification. These are good examples of dependency inversion, but `Services`
is intentionally WordCraft/egui-shaped and should not be copied into a screenplay engine.

There are two app-level dispatch methods. [`WordApp::run()`](../../crates/ui-egui/src/lib.rs:594)
is for user actions: it may open an input dialog, ask to save before destructive document actions,
fill clipboard parameters, and then execute. [`WordApp::execute()`](../../crates/ui-egui/src/lib.rs:641)
is for scripts and agents: it never asks first, handles `ui.*` commands and host-specific behavior,
then delegates engine commands to [`Session::run()`](../../crates/ui-egui/src/lib.rs:704). The
distinction is important. A screenplay app should expose an equivalent “user intent” versus
“automation intent” split, but dialogs must never be required for a programmatic command.

The canvas obtains a mutable layout from the session, paints pages and hit-tests positions. At the
end of canvas handling it invokes [`canvas_events()`](../../crates/ui-egui/src/canvas.rs:1341).
After a command changes the session, the next layout read sees a new revision and recomputes. UI
concerns such as focus restoration after ribbon commands are kept in
[`run_from_ribbon()`](../../crates/ui-egui/src/widgets.rs:118), rather than inside engine commands.

### 1.7 Ribbon, keymap and catalog

The keymap is independent of egui: [`KeyMap::resolve()`](../../crates/engine/src/keymap.rs:122)
normalizes a portable shortcut, applies user assignments/removals and returns the matching command
spec. Custom keymap input is bounded and malformed entries are dropped
([`KeyMap::from_value()`](../../crates/engine/src/keymap.rs:94)). The ribbon layout follows the same
command IDs and drops unknown commands on restore
([`RibbonLayout`](../../crates/engine/src/ribbon.rs:115)).

This is reusable metadata, but a screenplay app needs only a compact command palette/menu model.
The full Word feature catalog is a parity/reporting concern, not an engine requirement; a smaller
app can omit the catalog and use command locations only for menus and search.

### 1.8 CLI, control and MCP consumers

The CLI creates a [`Session`](../../apps/wordcraft-cli/src/main.rs:46), parses `--cmd` entries and
runs command IDs. Its `commands` mode serializes the registry, while `run` applies commands
headlessly and can save or print ([`wordcraft-cli` usage](../../apps/wordcraft-cli/src/main.rs:13)).

The desktop control server is a loopback JSON-lines transport with a per-window key. Its protocol
defines `engine.execute`, `engine.commands`, document/UI inspection, synthetic pointer/key/text
events and screenshots ([`docs/control-protocol.md`](../../docs/control-protocol.md:1)). The egui
adapter queues synthetic events in [`control::handle()`](../../crates/ui-egui/src/control.rs:107),
so `ui.key` and `ui.text` take the real frontend path rather than bypassing focus and shortcut
logic.

MCP provides two backends. [`Headless`](../../crates/mcp/src/lib.rs:24) runs a session in process;
`engine.execute` calls [`Session::run()`](../../crates/mcp/src/lib.rs:38), and render calls session
layout plus the renderer. [`Remote`](../../crates/mcp/src/backend.rs:24) sends the same methods to a
running app through the control channel. This gives screenplay automation a natural test split:
fast deterministic headless command tests, plus a smaller set of remote/UI-path acceptance tests.

## 2. One edit traced end to end

For a user typing a character in the document:

1. egui emits `Event::Text`. The focused canvas's [`canvas_events()`](../../crates/ui-egui/src/keys.rs:201)
   rejects control/modifier text, then calls [`WordApp::run()`](../../crates/ui-egui/src/lib.rs:594)
   with `text.insert` and `{"text": ...}`.
2. `WordApp::run()` performs user-only preparation (dialog/save checks and clipboard adaptation),
   then [`execute_user()`](../../crates/ui-egui/src/lib.rs:799) reaches
   [`WordApp::execute()`](../../crates/ui-egui/src/lib.rs:641).
3. `execute()` handles app-owned special cases and calls [`Session::run()`](../../crates/ui-egui/src/lib.rs:704).
4. `Session::run()` resolves the [`CommandSpec`](../../crates/engine/src/lib.rs:58), snapshots
   state, opens or joins the typing undo group, and invokes the `text.insert` implementation.
5. `text.insert` applies selection/overtype and autoformat policy, then mutates the document through
   the document editing primitives ([`insert()`](../../crates/engine/src/cmd/text.rs:88)).
6. On success, `Session::run()` increments the revision and dirty flag, clamps selection and
   finishes session bookkeeping. The cached layout is now stale because its revision key no longer
   matches.
7. The next canvas/layout pass calls [`Session::layout()`](../../crates/engine/src/session.rs:727),
   which recomputes the display layout and returns it to painting/hit testing. The UI may update
   caret geometry and status/focus state, but does not perform document repair.
8. The same edit can instead start as `ui.text` in control, an `engine.execute` request in MCP, or
   a CLI `--cmd`; those routes converge on the same engine command. Headless tests can inspect the
   document and render the page without starting egui.

For a drag or repeated geometry edit, the UI can call [`join_next_undo()`](../../crates/engine/src/session.rs:835)
between frames. The command still mutates and invalidates normally, but all frames collapse into one
undo step. The screenplay equivalent might be a multi-selection drag or a panel resize, although
panel-only changes should remain UI state and not enter document history.

## 3. Recommendations for a 20–40-command screenplay app

### 3.1 Retain

1. **Stable command IDs and one invocation contract.** Keep `command_id + JSON params + JSON result`
   as the public seam. Every command should document parameters, whether it mutates, and enablement.
   This supports menus, keyboard shortcuts, CLI, tests and future agents without coupling them to
   widget types.
2. **A reduced `CommandSpec`/registry.** Retain ID, label, shortcut, params, mutability, enabled
   predicate and handler. Drop Word's ribbon location or replace it with a lightweight menu group
   unless the screenplay app actually needs customizable ribbons.
3. **Central session transactions.** Retain pre-command snapshots, rollback on `Err`, panic-to-error
   containment, selection clamping, revision/dirty tracking and a single mutation boundary. Do not
   let UI code mutate the screenplay model directly.
4. **Typing-aware undo coalescing.** Retain a typing group, explicit split points on selection/caret
   movement, separate automatic-transformation undo where user expectations require it, and
   `join_next_undo` for gesture updates.
5. **Revision-keyed layout cache.** Retain a session-owned cache keyed by document revision and
   presentation inputs. A screenplay app may have simpler pagination, but it still needs reliable
   invalidation for page size, zoom-independent layout width, element styles, font changes and
   visibility modes.
6. **Portable keymap resolution.** Retain normalized platform-independent shortcuts, user overrides,
   removed built-ins and bounded preference loading. Keep egui/platform key conversion in the UI.
7. **Headless backend and inspect/render commands.** Retain command enumeration, document inspection,
   deterministic page/image rendering and in-process execution. These provide the fastest regression
   loop and an automation API for a small app.
8. **Injected host services.** Retain the dependency-injection idea for file dialogs, clipboard,
   downloads, printing and notifications, but define a small screenplay-specific trait or service
   struct rather than importing all WordCraft services.

### 3.2 Simplify or exclude

* Do not retain the full 422-command Word registry, parity catalog, ribbon customization, keytips,
  table/shape/object subsystems, mail merge, citations, Track Changes, equation editor or Word
  backstage unless the screenplay product has a concrete requirement.
* Do not put egui types, `Context`, `Ui`, `Response`, canvas rectangles, dialogs or host callbacks in
  the screenplay engine. The engine should compile and test without egui, as WordCraft's layering
  rule describes ([`docs/architecture.md`](../../docs/architecture.md:9)).
* Do not make commands open dialogs. Return `Params`/`Disabled` errors or a structured `pending`
  result only at the app boundary. WordCraft's distinction between [`run()`](../../crates/ui-egui/src/lib.rs:594)
  and [`execute()`](../../crates/ui-egui/src/lib.rs:641) is the model to preserve.
* Do not treat UI presentation state as document undo state. Screenplay view mode, sidebars,
  selected panel and zoom should live in UI/session presentation state; element-role changes and
  document edits belong in undo.

### 3.3 Suggested command inventory

A practical first cut is 30–36 commands, grouped by behavior rather than by widget:

* **Text/editing (8):** `text.insert`, `text.newParagraph`, `text.backspace`, `text.delete`,
  `text.tab`, `text.backTab`, `edit.cut`, `edit.paste`.
* **Selection/navigation (7):** caret left/right/up/down, home/end, select all, select word/line,
  next/previous screenplay element, find/find next.
* **Screenplay structure (7):** apply element type, cycle element type, insert scene heading,
  insert action, insert character, insert dialogue, insert transition/parenthetical.
* **Formatting/view (4–6):** toggle bold/italic if supported, show/hide page breaks, zoom, outline/
  navigator toggle, screenplay layout settings.
* **History (2):** undo, redo; optionally undo history.
* **Project/file (5–6):** new, open, save, save as, export PDF, inspect/project metadata.

The exact IDs are a product decision, but IDs should be stable and semantic. Element transitions
should be commands, not hard-coded side effects of a particular toolbar or text widget, so keyboard,
automation and UI buttons behave identically.

## 4. Headless and UI testing strategy

### 4.1 Engine-first tests

For each command, test successful mutation, disabled state, malformed parameters, selection result,
undo/redo and layout revision behavior. Use the pattern in
[`tests_typing.rs`](../../crates/engine/src/tests_typing.rs:11): feed the same primitive commands
that the keyboard uses, then assert paragraph/element structure and caret state. Add screenplay
parity tests for “typing a scene heading then Enter”, “cycling element type”, dialogue continuation,
blank-line behavior, and undo grouping.

Retain hostile-input coverage. The existing [`hostile_params_never_panic()`](../../crates/engine/src/tests.rs:1856)
iterates registered commands with junk values; a screenplay registry should do the same and assert
that every failure is an error with unchanged document/history. Add property tests for arbitrary
Unicode, byte/character boundary positions, huge text, invalid element IDs and repeated undo/redo.

For layout, assert that a post-edit layout is not the old cached revision, that an unchanged read
returns the same cached layout where appropriate, and that undo restores both text and caret/page
placement. Use engine rendering for stable pixel or structural snapshots, not screenshots of a
live desktop window.

### 4.2 Headless automation tests

Use the in-process MCP-style backend or a direct `Session` for command sequences. Enumerate commands
and verify metadata; execute a screenplay creation script; call document inspection; undo and redo;
then render a page. This tests the public automation seam without timing, focus or window-manager
noise. The existing [`Headless`](../../crates/mcp/src/lib.rs:24) backend is a concrete template.

For transport coverage, test the JSON-lines control server separately: authentication, malformed
JSON, line limits, request/reply shape, command errors and queued synthetic input. The documented
control protocol explicitly guarantees that failed commands leave the document unchanged
([`docs/control-protocol.md`](../../docs/control-protocol.md:12)).

### 4.3 UI-path tests

Use egui_kittest-style raw event frames for a small number of end-to-end cases: focus canvas,
send text/Enter/shortcut events, click a toolbar command, inspect the session, and compare a page
render or screenshot. The control API's [`ui.text`](../../crates/ui-egui/src/control.rs:207) and
[`ui.key`](../../crates/ui-egui/src/control.rs:200) are useful because they queue events that take
the real egui path; `ui.clickText` also exercises page hit testing
([`control::handle()`](../../crates/ui-egui/src/control.rs:152)).

Keep UI assertions about focus, visible controls, caret placement and selection—not screenplay
semantics already covered by engine tests. Run headless UI shots with fixed window size, fonts and
animation settings. Prefer structural `document.inspect` and engine `ui.render` output for behavior;
use full UI screenshots only for visual regressions.

## 5. Boundary risks and migration notes

The biggest reuse risk is accidentally importing `WordApp` as the application model. It mixes engine
session state with egui frame state, dialogs, platform services, automation queues and Word-specific
presentation. Reuse the lower-level command/session contract and implement a screenplay-specific
frontend adapter.

The second risk is external side effects inside commands. WordCraft's rollback protects in-memory
edits, but a command that writes a file, downloads bytes or invokes a printer cannot be rolled back
by restoring a document snapshot. For the screenplay app, keep save/export side effects in an
application service or make them explicit, idempotent commands with clear result status.

The third risk is ambiguous mutability. A command that changes screenplay element defaults, UI
preferences, document content or layout presentation must declare which state it changes. Use
separate preference/UI commands and document-edit commands; only the latter should create document
undo steps.

Finally, keep the “same command, different transport” rule visible in tests. A behavior is not
complete when a button works: the direct engine call, keyboard event, headless automation call and
remote control call should converge on the same command semantics, with only user-facing dialog and
focus policy differing at the UI boundary.

## Conclusion

WordCraft's reusable asset is not the ribbon; it is the command/session boundary. A screenplay app
should retain stable command metadata, centralized transactions, typing-aware undo, revision-keyed
layout invalidation, portable key resolution, injected host services and a headless automation
backend. It should replace the Word-sized registry and `WordApp` with a deliberately small command
set and a thin screenplay UI adapter. That preserves deterministic behavior across keyboard, menu,
CLI, MCP and tests while preventing egui and Word-specific workflow policy from leaking into the
screenplay engine.

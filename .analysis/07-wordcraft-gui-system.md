# WordCraft’s GUI system: a beginner’s tour

This guide explains the desktop GUI from the outside in. It is aimed at someone who is new to
Rust and wants a useful mental model rather than a list of every widget.

## The short answer: is there a generic GUI framework?

Yes. WordCraft uses a generic GUI toolkit, and then builds its own word-processor front end on top
of it:

| Layer | What it is responsible for |
|---|---|
| **egui** | The generic immediate-mode GUI toolkit: layout helpers, buttons, text fields, menus, input events, painting commands, focus, clipping and accessibility integration. |
| **eframe** | The application and window integration layer around egui. It starts the native or web event loop, owns the window/viewport, gathers platform input, calls the app, and presents egui’s output. |
| **winit** | The lower-level window/event abstraction used underneath the integration. It deals with windows, keyboard/mouse events, resizing, file drops and platform event loops. WordCraft also uses it directly for one Linux event-loop choice. |
| **wgpu** | The graphics backend used to send egui’s paint output to the GPU. It is the rendering path, not the place where WordCraft’s ribbon or document commands live. |
| **`ui-egui`** | WordCraft’s application-specific front end, written with egui and hosted by eframe. It defines `WordApp`, the ribbon, canvas, panes, dialogs, themes, key handling and translations. |

The dependency declarations make this boundary visible: the desktop application enables eframe’s
`wgpu`, accessibility and platform features in [`apps/wordcraft/Cargo.toml`](../apps/wordcraft/Cargo.toml:22),
while [`crates/ui-egui/Cargo.toml`](../crates/ui-egui/Cargo.toml:6) describes `ui-egui` as a
swappable front end. The architecture rule is even stronger: nothing below the UI layer depends on
egui, eframe, winit or rfd ([`docs/architecture.md`](../docs/architecture.md:9)).

So the answer is not “the translation files are the GUI framework.” Translation is one feature of
the application front end. **egui is the generic GUI toolkit, eframe is the application/window
integration layer around egui, and `ui-egui` is WordCraft’s application-specific front end built
with those libraries.**

## 1. Starting the desktop application

The desktop path begins in [`main()`](../apps/wordcraft/src/main.rs:235). Before a window exists,
it installs logging, parses options such as `--sample` and `--control`, prepares
[`eframe::NativeOptions`](../apps/wordcraft/src/main.rs:268), restores window geometry, and lets
the host choose platform services.

The `NativeOptions` value is the window’s starting recipe: title, size, minimum size, drag-and-drop,
title-bar choices and icon. On Windows, WordCraft can adjust the wgpu backend before eframe creates
it ([`graphics::configure()`](../apps/wordcraft/src/main.rs:283)). On Unix, it can ask winit to use
X11 when that is needed for file drag-and-drop ([`event_loop_builder`](../apps/wordcraft/src/main.rs:293)).
These are host concerns, not document-editor concerns.

The important hand-off is [`eframe::run_native()`](../apps/wordcraft/src/main.rs:309). Its creator
closure receives eframe’s creation context. WordCraft uses that context to:

1. choose a region and create a blank or sample document;
2. create an engine [`Session`](../apps/wordcraft/src/main.rs:318);
3. create [`WordApp::new()`](../apps/wordcraft/src/main.rs:320), injecting `Services`;
4. restore preferences and fonts;
5. optionally attach the localhost control channel; and
6. return the desktop wrapper around `WordApp`.

The wrapper is the local [`App`](../apps/wordcraft/src/main.rs:34) type. It implements
[`eframe::App`](../apps/wordcraft/src/main.rs:42), whose methods are the bridge between eframe’s
loop and WordCraft:

- [`App::logic()`](../apps/wordcraft/src/main.rs:43) performs work before the visible layout:
  macOS events, WordCraft’s per-frame logic, dialogs requested by the host, window geometry and
  close handling.
- [`App::raw_input_hook()`](../apps/wordcraft/src/main.rs:73) lets `WordApp` add synthetic input,
  which is how the control/headless path can behave like a real pointer or keyboard event.
- [`App::ui()`](../apps/wordcraft/src/main.rs:76) asks `WordApp` to describe the visible interface.
- [`App::on_exit()`](../apps/wordcraft/src/main.rs:80) finishes printing, saves preferences and
  removes the control key.

The exact method names are worth noticing: this version of eframe separates the pre-layout
`logic` pass from the `ui` draw/layout pass. The two together are one GUI frame.

## 2. What happens in one frame?

The reusable object passed through the frame is [`egui::Context`](../apps/wordcraft/src/main.rs:43).
Think of it as the conversation between the app and egui: it contains the current input, screen
size, focus and style information, and accepts requests such as “repaint soon,” “open a URL,” or
“change the viewport title.”

In [`WordApp::logic()`](../crates/ui-egui/src/lib.rs:1309), WordCraft first establishes application
conditions for the frame: it selects the current interface language, synchronizes options, installs
fonts and theme, applies interface zoom and animation settings, polls services and control requests,
handles dropped files, watches fonts, and schedules repaint/autosave work. This is state preparation;
it is not yet the main painting pass. The language and theme work is visible around
[`i18n::set_current()`](../crates/ui-egui/src/lib.rs:1311) and [`theme::apply()`](../crates/ui-egui/src/lib.rs:1341).

Then [`WordApp::ui()`](../crates/ui-egui/src/lib.rs:1497) lays out the window. In the normal editor
mode the order is intentionally readable:

1. title bar;
2. ribbon;
3. status bar;
4. side panes; and
5. a central panel containing the document canvas.

That sequence is in [`WordApp::ui()`](../crates/ui-egui/src/lib.rs:1517). Backstage replaces the
editor when it is active, and dialogs, popups, the built-in file browser, colours and alerts are
then shown above the main content. `ui-egui` is therefore not a translation layer over JSON: it is
the code that turns the current in-memory app state into egui layout and paint commands.

At the end of a frame, eframe takes the egui output and presents it in the native window. With the
wgpu feature enabled, wgpu is the part that turns those drawing commands into GPU work. You usually
do not need to learn graphics APIs to understand WordCraft UI code: an egui `Button`, layout
container, or `Painter` call is the interesting level; eframe/winit/wgpu are the machinery that
gets the result on screen.

## 3. Immediate mode without the jargon

egui is an **immediate-mode GUI**. In plain language, the app redraws the interface by describing
what should exist *this frame*, rather than constructing a permanent tree of widget objects and
waiting for those objects to repaint themselves.

For example, the application can say, conceptually:

```text
if the “Bold” button was clicked this frame:
    run the bold command
draw the Bold button
```

The button’s interaction result is returned while the button is being described. Its longer-lived
facts—selected ribbon tab, dialog contents, caret, document and so on—live in WordCraft state, not
inside an invisible widget tree.

This differs from a traditional retained-mode GUI, where you might create a `BoldButton` object,
attach a click callback to it, keep it in a widget hierarchy, and later mutate that object. egui
still remembers enough temporary information to make focus, IDs, text editing and keyboard input
work, but the application’s normal job is to describe the current frame again.

Immediate mode does **not** mean “stateless.” It means the *description and interaction check* are
repeated. WordCraft stores durable state in `WordApp`, `UiState`, `CanvasState` and `Session`.

## 4. What belongs to `ui-egui`?

The root of [`crates/ui-egui/src/lib.rs`](../crates/ui-egui/src/lib.rs:1) lists the front end’s
parts. The crate contains much more than a few translated labels:

- [`WordApp`](../crates/ui-egui/src/lib.rs:343) is the UI-facing application object. It owns the
  engine `Session`, UI preferences, canvas state, dialog/pane state, printing state, control state
  and injected services.
- [`Services`](../crates/ui-egui/src/lib.rs:88) is a set of host-provided hooks for file dialogs,
  printers, downloads, clipboard formats, animation preferences, fonts and default-app settings.
- [`UiState`](../crates/ui-egui/src/lib.rs:164) holds persistent interface choices: current tab,
  Backstage page, recent files, theme, interface size, language, window geometry, keytips and
  customized keyboard/ribbon settings.
- [`theme`](../crates/ui-egui/src/theme.rs:1) installs egui styling, fonts and WordCraft’s
  light/dark [`Tokens`](../crates/ui-egui/src/theme.rs:197). Tokens keep colours and spacing
  decisions coherent instead of scattering colour literals through every widget.
- [`ribbon`](../crates/ui-egui/src/ribbon.rs:1), [`chrome`](../crates/ui-egui/src/chrome.rs:1)
  and [`backstage`](../crates/ui-egui/src/backstage.rs:1) build the application chrome: tabs,
  groups, title bar, status bar and File/Home-style views.
- [`canvas`](../crates/ui-egui/src/canvas.rs:1) places pages, draws cached page textures, rulers,
  selection, caret, objects, tables, ink and comments, and handles page interaction.
- [`panes`](../crates/ui-egui/src/panes.rs:1) and the [`dialogs`](../crates/ui-egui/src/dialogs.rs:1)
  family provide navigation, styles, comments, review/options panes and modal forms.
- [`keys`](../crates/ui-egui/src/keys.rs:1) translates egui keyboard/IME events into command
  dispatch, including ordinary text, shortcuts, paste and equation editing.
- [`control`](../crates/ui-egui/src/control.rs:1) provides control-channel operations, UI
  inspection, screenshots, synthetic clicks, drags, keys and text.

The crate’s own tests include egui-kittest interaction tests. They exercise things such as page hit
testing and caret movement, preferences, dialogs, themes, menus and input without requiring a human
to click the real desktop window. The UI crate also includes a headless screenshot path; the control
module documents `ui.click`, `ui.key`, `ui.text`, `ui.screenshot` and `ui.render` at
[`control.rs`](../crates/ui-egui/src/control.rs:1).

## 5. How input reaches the document

The UI does not directly edit a JSON document. It presents an in-memory `Document` held by the
engine `Session`, and asks the session to perform commands.

### Typing one character

The flow for typing `a` is approximately:

1. winit/eframe gathers the physical keyboard or text/IME event.
2. egui places that event in the current frame’s input.
3. [`keys::canvas_events()`](../crates/ui-egui/src/keys.rs:199) sees an egui `Text` or IME commit
   event while the canvas has editing focus.
4. `ui-egui` calls [`WordApp::run()`](../crates/ui-egui/src/lib.rs:594) with the command id
   `text.insert` and a text parameter.
5. `WordApp::run()` applies UI-level policy first—pending dialogs, clipboard handling and save
   prompts—then [`WordApp::execute()`](../crates/ui-egui/src/lib.rs:641) passes the normal command
   to [`Session::run()`](../crates/engine/src/lib.rs:1).
6. The engine updates the document, selection, undo history and layout cache.
7. The next frame asks the updated session for layout data, draws the changed page and paints the
   caret in its new location.

The actual text branch is visible at [`keys.rs`](../crates/ui-egui/src/keys.rs:287), and the
command-to-session boundary is visible at [`lib.rs`](../crates/ui-egui/src/lib.rs:704). This common
command path is why a ribbon button, shortcut, CLI command, control request and MCP request can
reach the same editor behavior.

### Clicking a toolbar button

A ribbon button is an egui widget defined by WordCraft. egui detects the pointer press and returns a
clicked response. The ribbon code maps that button to a command id, checks enablement, and calls
`WordApp::run()`. A formatting button might therefore run `format.bold`; a button that needs user
input may instead open a `ui-egui` dialog first, and the dialog’s OK action later runs the command
with parameters.

The command registry and session, not the translation catalog, decide what the command means. The
button’s displayed label may be translated, but its stable id remains the same.

## 6. From layout data to painted pages

The editor canvas is a useful example of the boundary between application UI and rendering:

1. The engine/layout layers turn the document into pages containing positioned text, images, tables
   and other display items. The architecture summary calls this a display list
   ([`docs/architecture.md`](../docs/architecture.md:44)).
2. [`canvas::show()`](../crates/ui-egui/src/canvas.rs:680) obtains the session layout and decides
   page positions, columns, zoom, scrolling and visible bands.
3. Visible page content is rendered through `wordcraft-render`, cached as an egui texture, and
   painted by an egui [`Painter`](../crates/ui-egui/src/canvas.rs:907).
4. UI-only overlays—selection highlights, caret, rulers, object handles, comments, gridlines and
   ink controls—are painted on top.

This is why the page is not just a large text box. The document’s layout knows where content is;
the UI decides where that page appears in the viewport and adds interactive editor chrome.

The reverse direction is hit testing. A click first becomes a screen coordinate. The canvas maps it
to a page and document coordinate with [`page_at()`](../crates/ui-egui/src/canvas.rs:1686), then
asks layout for the nearest document position with `layout.hit`. The normal pointer path is in
[`canvas::mouse()`](../crates/ui-egui/src/canvas.rs:1739). For caret display, the UI asks layout
for [`caret_on()`](../crates/ui-egui/src/canvas.rs:1149), converts the returned page position to
screen coordinates, and paints the caret. The control path exposes the same geometry through
[`caret_screen()`](../crates/ui-egui/src/canvas.rs:2387) and `pos_from_screen`.

In short: **layout tells the UI where things are; the UI asks layout where a screen click means.**

## 7. Platform services stay at the edge

The lower crates should not know whether the user is on macOS, Windows, Linux or the web. The
desktop app supplies closures and workers in [`services()`](../apps/wordcraft/src/main.rs:205):
native file dialogs, printers, clipboard text/HTML/RTF/pictures, default-app integration and the
OS animation setting. `WordApp` can request one of these through `Services` without importing
platform APIs into the engine or layout crates.

This is dependency injection: the UI receives a capability instead of constructing a macOS dialog
or Linux printer itself. It also explains why the same `ui-egui` crate can be used by the web app or
headless tests with different hooks. The app-specific platform modules—such as
[`file_dialogs.rs`](../apps/wordcraft/src/file_dialogs.rs:1) and the macOS Apple-event handler—stay
in the desktop application because they are OS integration, not generic document behavior.

## 8. Localization is data, not a GUI framework

WordCraft does have translation data in the UI crate. [`i18n/mod.rs`](../crates/ui-egui/src/i18n/mod.rs:1)
explains the design: English strings in UI code are lookup keys, and one catalog per language maps
those keys to display text. Catalogs are embedded from TSV files such as
[`zh-hans.tsv`](../crates/ui-egui/src/i18n/zh-hans.tsv:1). The language registry, locale matching,
fallback-to-English behavior and formatting helpers all live in this module.

The UI selects the language at the start of a frame with [`Lang::from_pref()`](../crates/ui-egui/src/i18n/mod.rs:202)
and [`set_current()`](../crates/ui-egui/src/i18n/mod.rs:311). UI code can then use the `tl!` helper
declared at [`lib.rs`](../crates/ui-egui/src/lib.rs:15). Translation changes what the user sees:
button labels, menus, messages, ribbon locations and built-in style names. It does **not** change
command ids, document text, file names, JSON control requests, CLI syntax or MCP behavior
([`i18n/mod.rs`](../crates/ui-egui/src/i18n/mod.rs:1)).

That distinction matters: egui knows how to lay out and paint a button, but it does not know that
“Bold” should be translated into German, nor what WordCraft’s `format.bold` command does. WordCraft
chooses a translated string, hands that string to egui, and keeps the stable command id underneath.

Persistence is separate too. `ui.json` stores interface preferences such as theme, tab, language
and window geometry; document persistence uses engine I/O and formats such as DOCX or JSON. The GUI
is not a translation layer for JSON and does not turn the visible UI into a JSON document. It
presents the in-memory document/session; persistence reads or writes data separately.

## 9. Three kinds of state

It helps to keep these separate:

### Document state

The actual authored content: paragraphs, runs, tables, styles, sections, images, comments and so
on. It belongs to the document model and is what file formats preserve.

### Session/editor state

The working editor around that document: selection and caret, undo history, current path and dirty
flag, layout cache, view mode, zoom, proofing preferences and command registry. It belongs to the
engine [`Session`](../crates/engine/src/lib.rs:1). Some preferences are copied to/from UI preferences
when the app starts or exits, but the session owns the live editing behavior.

### UI state

How the application is presented: selected ribbon tab, Backstage visibility, open dialog, navigation
pane tab, theme, interface scale, keytips, window geometry, recent files and canvas interaction
bookkeeping. Persistent parts are represented by [`UiState`](../crates/ui-egui/src/lib.rs:164);
short-lived parts include `CanvasState`, dialog forms and focus flags. UI state can change without
changing the document—for example, opening the Navigation pane or switching the interface to dark
mode.

## 10. A sensible beginner reading path

Read in this order, using the tests as executable examples rather than trying to understand all
13,000-plus lines of UI code at once:

1. [`docs/architecture.md`](../docs/architecture.md:9) for layers and the document/layout/render
   data flow.
2. [`apps/wordcraft/src/main.rs`](../apps/wordcraft/src/main.rs:235) for startup, `App` and
   `run_native`.
3. [`crates/ui-egui/src/lib.rs`](../crates/ui-egui/src/lib.rs:343) for `WordApp`, `Services`,
   `UiState`, `run`, `logic` and `ui`.
4. [`crates/ui-egui/src/keys.rs`](../crates/ui-egui/src/keys.rs:199) for text, IME and shortcuts.
5. [`crates/ui-egui/src/ribbon.rs`](../crates/ui-egui/src/ribbon.rs:1) and
   [`crates/ui-egui/src/dialogs.rs`](../crates/ui-egui/src/dialogs.rs:1) for ordinary application
   UI.
6. [`crates/ui-egui/src/canvas.rs`](../crates/ui-egui/src/canvas.rs:680) for the page, caret,
   paint and hit-testing loop.
7. [`crates/ui-egui/src/i18n/mod.rs`](../crates/ui-egui/src/i18n/mod.rs:1) and
   [`crates/ui-egui/src/theme.rs`](../crates/ui-egui/src/theme.rs:1) for localization and visual
   styling.
8. The egui-kittest tests in [`crates/ui-egui/src/lib.rs`](../crates/ui-egui/src/lib.rs:2896) and
   [`crates/ui-egui/src/canvas.rs`](../crates/ui-egui/src/canvas.rs:2560) for examples of driving
   frames, clicking coordinates, checking caret geometry and verifying state changes. For typing
   parity, read [`docs/typing-parity.md`](../docs/typing-parity.md:1) and the engine typing tests
   referenced there.
9. [`docs/control-protocol.md`](../docs/control-protocol.md:1) for how an external tool drives the
   same UI/engine boundary.

## 11. What a small screenplay app should copy—and avoid

A small screenplay editor can keep the same healthy boundary:

```text
desktop app (window + platform services)
        ↓
generic egui widgets and painting
        ↓
small screenplay UI front end
        ↓
session/editor commands
        ↓
screenplay document and layout
```

It would need a `Session`-like editor object, a document model, a few commands (`text.insert`,
paragraph/scene formatting, save), a focused canvas or text editor, a theme, and perhaps a small
`Services` struct for file picking and clipboard access. Keep the command boundary even if there
are only ten commands; it makes keyboard, buttons and automation agree.

It should **not** copy WordCraft’s full complexity just because the architecture can support it.
Avoid starting with every ribbon tab, Backstage, 54-dialog family, multi-page object canvas, mail
merge, tracked changes, accessibility descriptions, custom keytips and localization catalog. Start
with one window, one screenplay text view, a simple toolbar, a caret/selection, undo, save/open and
one or two dialogs. Add state only when a feature needs it. The useful lesson is the separation of
responsibilities, not the size of WordCraft’s feature inventory.

## Glossary and mental model

- **egui:** the generic GUI toolkit. WordCraft calls it to lay out controls, read input and emit
  paint commands.
- **eframe:** the app/window integration layer around egui. It runs the native/web loop and presents
  egui output.
- **winit:** the cross-platform window and event abstraction underneath the integration.
- **wgpu:** the graphics backend that presents paint output using the GPU when configured.
- **`ui-egui`:** WordCraft’s own front end built with egui and hosted by eframe.
- **Immediate mode:** describe and interact with the current frame; keep meaningful state in your
  application rather than in a permanently mutated widget tree.
- **Context:** egui’s per-application conversation object: input, screen information, style and
  requests for repaint or viewport changes.
- **Layout:** converting document content into positioned pages and items, and answering “what is
  under this point?”
- **Painter:** egui’s low-level drawing helper used by WordCraft for page textures and overlays.
- **Service injection:** passing platform capabilities into the UI so lower layers stay portable.
- **Catalog:** a language’s mapping from stable English UI keys to display strings.

The mental model is: **eframe opens the stage, winit delivers the outside-world events, egui is the
generic stage toolkit, wgpu helps light the stage, and `ui-egui` directs the WordCraft play. The
engine/session owns what the document means; the UI decides how it looks and how the user reaches
those commands. Localization changes the labels on the controls, not the controls’ identity or the
document’s meaning.**

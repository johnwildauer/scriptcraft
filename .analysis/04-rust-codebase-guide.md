# A friendly tour of the WordCraft Rust codebase

This is a map for someone who has written a small Rust program, but has not yet opened a large Rust
project. You do not need to understand every feature in WordCraft. The useful first question is:
**what is the path from a user's action to a changed document and a new picture on screen?**

The source links below are places to peek, not homework to memorize. Lines may move as the project
changes.

## 1. The whole app in one story

Imagine WordCraft as a workshop:

1. A file is opened, or a blank/sample document is made.
2. The document is kept as structured information: paragraphs, text runs, tables, sections, and so
   on. This is the [`Document`](../crates/doc/src/lib.rs:563), not a screenshot of a page.
3. The current editing situation is kept in a [`Session`](../crates/engine/src/session.rs:322): the
   document, cursor/selection, undo history, view settings, and other working information.
4. A click, key press, CLI request, or automation request becomes a named command. For example,
   “insert this text” is sent through [`Session::run()`](../crates/engine/src/session.rs:1116).
5. The command changes the document. WordCraft can take a cheap undo snapshot first, so an edit can
   be reversed.
6. The layout code measures the changed document and works out lines, page breaks, positions, and
   hit-test information. It also makes drawing instructions: text here, line there, image there.
7. The user interface reads those results and paints the page in the window. PDF export uses the
   same layout idea without needing a window.
8. Save asks the file-format code to turn the structured document into `.docx`, `.txt`, `.pdf`, or
   another format.

In shorthand:

```text
file bytes -> Document -> Session -> command -> layout -> drawing -> window
                                      \\-> save -> file bytes
```

The repository's shorter version of this story is in [`docs/architecture.md`](../docs/architecture.md:36).

### What happens when the desktop app starts?

The desktop program begins at [`main()`](../apps/wordcraft/src/main.rs:235). It reads command-line
arguments, sets up logging, loads preferences and fonts, and asks the window library to create a
window. It then makes a session, puts that session inside the UI app, and starts the event loop.

An **event loop** is simply the repeating conversation with the operating system: “did the user
click, type, resize, or close the window?” On each turn, the UI handles input, runs commands when
needed, asks for fresh layout, and draws again. The small desktop adapter in [`App`](../apps/wordcraft/src/main.rs:34)
passes those turns to [`WordApp`](../crates/ui-egui/src/lib.rs:1).

The CLI and web app start differently, but reuse the same document and engine code. The CLI is a
particularly calm place to study because it can open a file, run commands, and save without making a
window: see [`apps/wordcraft-cli/src/main.rs`](../apps/wordcraft-cli/src/main.rs:1). The browser
entry point is [`apps/wordcraft-web/src/main.rs`](../apps/wordcraft-web/src/main.rs:1).

## 2. The folders in everyday language

| Place | Think of it as |
|---|---|
| [`Cargo.toml`](../Cargo.toml:1) | The top-level table of contents and shared Cargo settings. |
| [`apps/`](../apps:1) | Finished programs people can run: desktop, command line, and web. |
| [`crates/`](../crates:1) | Reusable pieces of WordCraft. These are custom packages written for this project, not automatically packages downloaded from the internet. |
| [`xtask/`](../xtask:1) | A small helper program for project checks such as `cargo xtask ci`. |
| [`assets/`](../assets:1) | Fonts, icons, dictionaries, and other non-Rust inputs. |
| [`docs/`](../docs:1) | The project's maintained explanations and protocols. |
| [`analysis/`](../analysis:1) | Research and teaching notes, including this file. |
| [`Cargo.lock`](../Cargo.lock:1) | Cargo's recorded choices of dependency versions. Cargo writes it; people normally do not edit it by hand. |
| `target/` | Cargo's disposable build shelf. It contains compiled results, not source code. |

The names `apps` and `crates` are helpful labels, not special Rust magic. Cargo learns what exists
from manifests and source roots.

## 3. Cargo, packages, crates, and modules

### Cargo is your workshop assistant

Cargo is Rust's build tool and package manager. It can create a project, download outside libraries,
compile code in the right order, run tests, and run the finished program. Commands you will use here
include:

```text
cargo check                 # compile-check without making a final executable
cargo test                  # compile and run tests
cargo run -p wordcraft      # run the desktop package
cargo run -p wordcraft-cli -- commands
```

The root [`Cargo.toml`](../Cargo.toml:1) declares a **workspace**. A workspace is a group of related
Cargo packages that live together and share settings and a build folder. WordCraft's workspace
includes everything under `crates/`, everything under `apps/`, and [`xtask`](../xtask/Cargo.toml:1).

### The names that sound alike

Use this mental picture: a **package** is a project box, a **crate** is one thing Cargo compiles
inside that box, and a **module** is a room inside that thing.

* A **package** has a [`Cargo.toml`](../apps/wordcraft/Cargo.toml:1). It gives the package a name and
  lists what it needs. The desktop package is named `wordcraft`.
* A **library crate** is reusable code. Its usual starting file is [`src/lib.rs`](../crates/engine/src/lib.rs:1).
  Other crates can call its public functions and types.
* A **binary crate** becomes a runnable program. Its usual starting file is [`src/main.rs`](../apps/wordcraft/src/main.rs:235),
  containing [`main()`](../apps/wordcraft/src/main.rs:235).
* A package can contain a library, a binary, or both. In this repository, the packages under
  `crates/` are mostly libraries, while the packages under `apps/` are programs.
* A **module** is a way to split one crate into understandable rooms. [`mod session;`](../crates/engine/src/lib.rs:19)
  tells Rust to include [`session.rs`](../crates/engine/src/session.rs:1). A module is not a package
  and does not get its own `Cargo.toml`.

So, yes: the crates in WordCraft's `crates/` folder are bespoke code made for WordCraft. For
example, [`wordcraft-doc`](../crates/doc/Cargo.toml:1) holds the document model, and
[`wordcraft-engine`](../crates/engine/Cargo.toml:1) coordinates editing. They connect because one
package lists another under `[dependencies]`, often using a local `path` or the workspace shorthand.

There are also outside crates. `serde`, `egui`, and `zip` are examples listed in the shared
dependencies in [`Cargo.toml`](../Cargo.toml:13). Cargo downloads and compiles those; WordCraft's
own crates are already in this checkout.

### How you would make your own crate

For reusable code:

```text
cargo new sentence-tools --lib
```

Cargo makes a new package with a [`src/lib.rs`](../crates/engine/src/lib.rs:1). Put a public function
there, such as `pub fn word_count(text: &str) -> usize`, and another package can list
`sentence-tools` as a dependency and call it.

For a runnable program:

```text
cargo new note-taker
cargo run
```

This makes [`src/main.rs`](../apps/wordcraft/src/main.rs:235) and runs its [`main()`](../apps/wordcraft/src/main.rs:235).
If the program needs reusable code, put that code in a library crate (or a `src/lib.rs` in the same
package) and let the binary be a small front door.

You would use a workspace when several related packages should be developed together. A tiny app
does not need WordCraft's number of packages just because WordCraft has them.

## 4. How the WordCraft pieces connect

WordCraft is large because it promises many formats, platforms, and Word-like features. Its broad
shape is a stack of jobs:

1. [`geom`](../crates/geom/src/lib.rs:1) handles simple measurements and rectangles.
2. [`doc`](../crates/doc/src/lib.rs:1) is the lasting meaning of the document: text, paragraphs,
   tables, styles, and edits.
3. [`layout`](../crates/layout/src/lib.rs:1) decides how that meaning fits onto pages and produces
   drawing instructions.
4. [`docx`](../crates/docx/src/lib.rs:1), [`formats`](../crates/formats/src/lib.rs:1), and
   [`docbin`](../crates/docbin/src/lib.rs:1) translate between file formats and the document model.
5. [`render`](../crates/render/src/lib.rs:1) paints layout results; [`pdf`](../crates/pdf/src/lib.rs:1)
   exports them.
6. [`engine`](../crates/engine/src/lib.rs:1) keeps a live editing session and gives actions names.
7. [`ui-egui`](../crates/ui-egui/src/lib.rs:1) draws controls and sends user actions to the engine.
8. The app packages supply the desktop window, command line, or browser surroundings.

The important rule is that a lower, simpler piece does not need to know about the window. The
document model can therefore be tested by itself, used by the CLI, and reused on the web. This is a
useful separation for a small app too: keep your data and editing rules apart from buttons when you
can. Do not copy the whole stack unless your app really needs it.

## 5. A gentle way to read the code

Do not open the biggest file first. Take one path:

1. Start at the root [`Cargo.toml`](../Cargo.toml:1). Notice the workspace members and the shared
   dependencies.
2. Choose the CLI before the desktop app. Open its [`Cargo.toml`](../apps/wordcraft-cli/Cargo.toml:1)
   and then [`src/main.rs`](../apps/wordcraft-cli/src/main.rs:1). It has fewer operating-system
   details.
3. Choose one library it uses, such as [`crates/engine/Cargo.toml`](../crates/engine/Cargo.toml:1).
   Look at its name and dependencies: those are the other pieces it is allowed to call.
4. Open [`crates/engine/src/lib.rs`](../crates/engine/src/lib.rs:1). Read the module list and the
   public types near the top. A line such as [`pub mod cmd;`](../crates/engine/src/lib.rs:10) points
   to another room in the crate.
5. Follow one room, such as [`session.rs`](../crates/engine/src/session.rs:322), rather than trying
   to read the entire crate. Then look at one command in [`crates/engine/src/cmd/`](../crates/engine/src/cmd/mod.rs:57).
6. Read nearby tests. A test often says more clearly than a helper function what behavior matters.
   Tests inside source are marked with [`#[cfg(test)]`](../crates/engine/src/lib.rs:167); separate
   integration tests live in a package's `tests/` directory, such as [`closed_stdout.rs`](../apps/wordcraft-cli/tests/closed_stdout.rs:1).

When lost, ask three questions: **what data enters this function, what does it return, and who calls
it?** Use search to find the type or function's definition and its callers. You are building a route,
not reading a novel from page one.

## 6. Rust ideas you need for this tour

### Ownership and borrowing

Rust gives each value a clear owner. When the owner goes away, the value can be cleaned up. A value
can be temporarily **borrowed** for reading or changing without handing over ownership. A read-only
borrow looks like [`&T`](../crates/doc/src/lib.rs:72); an exclusive changeable borrow looks like
[`&mut T`](../crates/doc/src/lib.rs:72). Rust checks that these temporary references cannot outlive
the value or conflict in unsafe ways.

You will often see a function borrow a document to inspect it, then borrow it mutably to edit it.
This is why Rust code may pass references instead of copying a whole document.

### Structs and enums

A **struct** groups named pieces of data, like a form with fields. [`Session`](../crates/engine/src/session.rs:323)
is a struct with fields for the document, selection, history, and layout cache.

An **enum** says “this value is one of these alternatives.” [`Block`](../crates/doc/src/lib.rs:78)
is either a paragraph or a table. Code must choose the correct case before using it, which is useful
for document data that has several possible shapes.

### `Result` and errors

[`Result<T, E>`](../crates/doc/src/lib.rs:72) is Rust's honest answer format: `Ok(T)` means success
and `Err(E)` means something went wrong. Opening a damaged file or receiving bad command parameters
should produce an error, not make the app crash. The [`?`](../crates/engine/src/io.rs:109) operator
means “if this is an error, return it to my caller.” WordCraft defines readable errors such as
[`DocError`](../crates/doc/src/lib.rs:60) and [`CmdError`](../crates/engine/src/lib.rs:38).

You will see rules against `unwrap()` and `expect()` in production code. They mean “I promise this
cannot fail”; real documents and real users make that promise unsafe. A beginner's good habit is to
handle the error, return it, or show a useful message.

### Traits, only when a shared ability is needed

A **trait** is a named ability, similar to “anything that can be displayed” or “anything that can
act as an app.” A type can implement that ability. The window layer implements the [`eframe::App`](../apps/wordcraft/src/main.rs:42)
trait so the window library knows how to ask it for input and drawing. You do not need to design
traits everywhere; introduce one when several different types genuinely need the same behavior.

### `Arc` and WordCraft's snapshots

[`Arc<T>`](../crates/doc/src/lib.rs:47) is a shared owner with a reference count. It lets several
parts of a program hold the same read-only data without making a full copy immediately. WordCraft
stores blocks behind `Arc`, so an undo snapshot can share most of the old document. When an edit needs
to change a shared block, [`Arc::make_mut`](../crates/doc/src/lib.rs:5) makes a private copy of that
piece first. This is why WordCraft can keep history without copying every paragraph for every key.

For a small single-window app, start with ordinary owned values. Learn `Arc` when you have a clear
need to share a snapshot or data between threads; it is not a better `clone()` for everything.

### `cfg` and different platforms

[`#[cfg(...)]`](../apps/wordcraft/src/main.rs:13) is a compile-time choice. It tells Rust to include
some code only on macOS, Windows, or WebAssembly. For example, the desktop app includes Apple event
code only on macOS, while the web package chooses browser startup for `wasm32`.

This differs from an ordinary `if`: code excluded by `cfg` is not compiled for that target at all.
Use this when the operating system really supplies different services, not to hide ordinary program
logic.

## 7. One edit, followed all the way through

Suppose the user types “Hi”. The exact UI path has many details, but the useful shape is:

1. The UI notices text input.
2. It chooses a command ID and parameters. WordCraft's command description is [`CommandSpec`](../crates/engine/src/lib.rs:58).
3. The command registry finds that ID. The same idea works for a ribbon button, shortcut, CLI
   request, or control request.
4. [`Session::run()`](../crates/engine/src/session.rs:1116) checks whether the command is available,
   takes the needed history information, and changes the [`Document`](../crates/doc/src/lib.rs:563).
5. Layout turns the changed document into lines and pages. The layout crate describes its drawing
   items in [`display`](../crates/layout/src/display.rs:1).
6. The UI paints those items. The cursor and selection are also calculated from the same document
   and layout data, so clicking a page can find a text position.
7. Saving passes the document to a writer chosen from the file name. The format dispatch starts in
   [`engine::io`](../crates/engine/src/io.rs:1).

The UI is therefore not the document. It is one way to look at and control the document. That is why
the CLI can use the same commands without pretending to be a window.

## 8. What to borrow for a smaller application

For a small notes, drawing, or writing app, these ideas are worth copying:

* Keep the important data in a plain model that does not know about buttons.
* Put edits in a few understandable functions or commands.
* Keep file reading and writing at an edge of the program.
* Make drawing a view of the model, rather than the only place where the model exists.
* Add tests next to the model and edit rules.

Do not copy WordCraft's full number of crates, file formats, command catalog, platform adapters,
font system, or undo machinery on day one. Start with one package, a small model, one window, and a
save format. Split a piece into a library crate when it becomes useful independently or when the
boundary makes the code easier to test. Architecture is a tool for keeping a program understandable,
not a contest to produce the most folders.

## 9. A practical learning and development path

1. Learn enough Rust to create a package, define a struct and enum, match on an enum, borrow a value,
   and return a [`Result`](../crates/doc/src/lib.rs:72).
2. Make a tiny command-line notes program. Store notes in a `Vec<String>`, print them, and save plain
   text. Do not start with a GUI.
3. Add tests for adding, deleting, loading, and malformed input. Practice handling errors instead of
   using `unwrap()`.
4. Read the small WordCraft model crate: [`crates/doc/src/lib.rs`](../crates/doc/src/lib.rs:1).
   Notice how its data types describe a document independently of the window.
5. Read one WordCraft flow in the CLI, then trace an edit through [`engine`](../crates/engine/src/lib.rs:1)
   and [`layout`](../crates/layout/src/lib.rs:1).
6. Run small checks while exploring:

   ```text
   cargo check --workspace
   cargo test --workspace
   cargo run -p wordcraft-cli -- commands
   cargo run -p wordcraft -- --sample
   ```

7. Only after a change is understandable, use the full project gate: `cargo xtask ci`. It checks
   formatting, warnings, tests, assets, layer rules, and the web build; see [`xtask/src/main.rs`](../xtask/src/main.rs:151).

## Short glossary

* **Cargo:** Rust's builder, test runner, and package manager.
* **Package:** A project box described by a `Cargo.toml` file.
* **Crate:** One piece Cargo compiles; it can be a reusable library or a runnable binary.
* **Library crate:** Reusable code, normally starting at `src/lib.rs`.
* **Binary crate:** A runnable program, normally starting at `src/main.rs`.
* **Module:** A room inside a crate used to organize code.
* **Dependency:** Code this package is allowed to use, either from this workspace or downloaded from
  elsewhere.
* **Workspace:** Several related packages managed together by one root `Cargo.toml`.
* **Struct:** A type made of named fields.
* **Enum:** A type that can be one of several named cases.
* **Borrow:** Temporary access to a value without taking it away from its owner.
* **Trait:** A named shared ability that a type can implement.
* **`Result`:** A value that is either success or an error.
* **`Arc`:** A reference-counted shared owner, useful here for document snapshots.
* **`cfg`:** A compile-time choice for a platform or build situation.
* **Session:** WordCraft's live editing context around a document, selection, history, and view.
* **Layout:** Working out where document content fits on pages and what should be drawn.

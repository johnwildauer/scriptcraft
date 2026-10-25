# Stage 2 — Testing, safety, and portability lessons

## Scope and method

This report separates observed repository facts from recommendations for a small screenplay-writing
application. It focuses on practices that protect a writer's in-memory work, persisted files, and
cross-platform behavior—not on reproducing WordCraft's feature count. Sources are the repository's
agent rules, manifests, implementation, tests, and committed architecture summary.

## Executive assessment

WordCraft treats an editor as safety-critical software: malformed commands and documents are hostile
inputs; failed edits are transactions; escaped panics are converted to errors; and tests cover both
semantic editing behavior and application boundaries. Those ideas are directly valuable to a
screenplay app. In particular, the small app should adopt:

1. a no-panic production policy with strict linting;
2. command validation before mutation;
3. command-level rollback for document, selection, and undo state;
4. regression tests for every data-loss or crash bug;
5. deterministic model and pagination tests;
6. persistence/recovery tests that simulate interrupted saves and corrupt inputs; and
7. headless command tests plus a small number of real UI smoke tests.

The app should not copy the full parity catalog, hundreds of Word-specific interaction cases, MCP,
multiple legacy formats, system-font substitution, or a repository-wide `xtask` quality framework
until those capabilities are product requirements. The safety boundary is small; WordCraft's parity
machinery is large because its target is Microsoft Word.

## 1. Observed facts

### 1.1 Safety policy and linting

The repository's [`AGENTS.md`](../../AGENTS.md:9) makes “never crash” outrank feature work because a
crash can lose writing. It prohibits panics and unsafe code in non-test code, requires errors to flow
through [`Result<T, E>`](../../AGENTS.md:12), treats file/command/MCP numbers as hostile, and requires
regression tests for crash fixes ([`AGENTS.md`](../../AGENTS.md:9)-[`AGENTS.md`](../../AGENTS.md:16)).
The same rules identify [`Session::run()`](../../crates/engine/src/session.rs:1116) as a last-resort
guard, explicitly not permission to write panic-prone code.

The workspace forbids [`unsafe_code`](../../Cargo.toml:78) and enables workspace lints; individual
crate roots add deny-level Clippy checks for `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, and
`unreachable` (for example [`crates/engine/src/lib.rs`](../../crates/engine/src/lib.rs:1)). Test code
is deliberately exempted from those particular convenience restrictions by
[`clippy.toml`](../../clippy.toml:1). The distinction is practical: production paths must report
recoverable failures, while tests may use assertions and test-only setup shortcuts.

The quality command [`cargo xtask ci`](../../xtask/src/main.rs:151) runs formatting, workspace Clippy
with warnings denied, all workspace tests, asset attribution, dependency-layer checks, and the WASM
check ([`xtask/src/main.rs`](../../xtask/src/main.rs:20)-[`xtask/src/main.rs`](../../xtask/src/main.rs:27),
[`xtask/src/main.rs`](../../xtask/src/main.rs:151)-[`xtask/src/main.rs`](../../xtask/src/main.rs:180)).

### 1.2 Transactional command execution and panic recovery

The central [`Session::run()`](../../crates/engine/src/session.rs:1116) validates command identity and
enablement, performs protection checks, snapshots mutable state, establishes undo checkpoints, and
dispatches the command. The dispatch is wrapped in
[`std::panic::catch_unwind`](../../crates/engine/src/session.rs:1193); an escaped panic becomes a
[`CmdError::Failed`](../../crates/engine/src/session.rs:1197) rather than reaching the UI. On an
ordinary error—or a converted panic—the session restores the document, selection, history, redo
stack, typing state, eviction count, and column selection ([`Session::run()`](../../crates/engine/src/session.rs:1240)-[`Session::run()`](../../crates/engine/src/session.rs:1254)).

The repository tests the contract directly. [`failed_command_leaves_document_unchanged()`](../../crates/engine/src/tests.rs:1845)
checks invalid formatting, unknown commands, and missing text parameters. The broader
[`hostile_params_never_panic()`](../../crates/engine/src/tests.rs:1856) enumerates every registered
command, supplies null, missing, wrongly typed, out-of-range, and huge values, then exercises
selection clamping and layout ([`tests.rs`](../../crates/engine/src/tests.rs:1857)-[`tests.rs`](../../crates/engine/src/tests.rs:1879)). File commands and picture/file insertion are
excluded because they need external I/O, which makes the boundary explicit rather than pretending
the test is comprehensive.

The snapshot API also documents what it does and does not cover. [`Session::edit_snapshot()`](../../crates/engine/src/session.rs:978)
and [`Session::restore()`](../../crates/engine/src/session.rs:1073) preserve document/edit history
state, but deliberately exclude path, clipboard, view, macros, and other host/UI state. This is an
important limitation: in-memory rollback cannot undo a filesystem write, network request, or OS
clipboard side effect.

### 1.3 Model, typing, layout, and property-oriented tests

The committed architecture summary reports 821 ordinary test functions and 10 property-test blocks
covering model, layout, format round trips, hostile input, typing parity, bidi, UI interaction, and
MCP acceptance ([`docs/architecture.md`](../../docs/architecture.md:51)-[`docs/architecture.md`](../../docs/architecture.md:61)).
The workspace includes [`proptest`](../../Cargo.toml:38), and hostile-input tests appear at several
boundaries: random/truncated legacy files, malformed drawing data, hostile layout properties, and
format importers. These tests are not just “does the output look right”; they check bounded work,
finite coordinates, safe rejection, and absence of panics.

The model tests establish invariants such as a new document containing one paragraph and valid
sections ([`new_document_is_valid()`](../../crates/doc/src/tests.rs:3)), shallow clone behavior for
cheap snapshots ([`clone_is_shallow()`](../../crates/doc/src/tests.rs:93)), and JSON persistence
round trips ([`json_round_trip()`](../../crates/doc/src/tests.rs:104)).

Layout tests are mostly deterministic fixtures built in code. For example,
[`tests_pages.rs`](../../crates/layout/src/tests_pages.rs:1) creates long bodies, headers, footers,
and notes, then asserts page counts, bounds, ordering, and caret locations
([`a_tall_footer_pushes_the_body_up()`](../../crates/layout/src/tests_pages.rs:56)-[`caret_in_a_continued_footnote_is_found_on_its_page()`](../../crates/layout/src/tests_pages.rs:127)).
The main layout tests also cover empty documents, wrapping, pagination, hit testing, hostile values,
and performance-sensitive cases ([`layout/src/tests.rs`](../../crates/layout/src/tests.rs:14)-[`layout/src/tests.rs`](../../crates/layout/src/tests.rs:29),
[`layout/src/tests.rs`](../../crates/layout/src/tests.rs:1142)-[`layout/src/tests.rs`](../../crates/layout/src/tests.rs:1165)).

Typing behavior is specified in a document and pinned by executable tests. [`docs/typing-parity.md`](../../docs/typing-parity.md:5)
requires changing the prose and [`tests_typing.rs`](../../crates/engine/src/tests_typing.rs:1)
together; examples include list transitions, undo grouping, AutoCorrect, and Enter/Backspace behavior
([`docs/typing-parity.md`](../../docs/typing-parity.md:11)-[`docs/typing-parity.md`](../../docs/typing-parity.md:34)).
This prevents a UI refactor from silently changing writing semantics.

### 1.4 CLI, headless, and UI testing

The CLI runs the same command/session core headlessly, while MCP has headless and remote backends;
the architecture summary identifies MCP acceptance tests and the UI layer uses `egui_kittest`
([`docs/architecture.md`](../../docs/architecture.md:51)-[`docs/architecture.md`](../../docs/architecture.md:56)).
The agent rules document a control channel and a headless [`ui_shot`](../../AGENTS.md:54) example for
driving the real frontend and taking screenshots without focus stealing. This creates a useful split:
fast deterministic command tests for most behavior, plus a smaller set of frontend-path tests.

The CLI has an integration regression for a real process boundary:
[`commands_json_into_a_closed_pipe_exits_quietly()`](../../apps/wordcraft-cli/tests/closed_stdout.rs:12)
and its MCP analogue assert that a client closing stdout does not produce a panic
([`closed_stdout.rs`](../../apps/wordcraft-cli/tests/closed_stdout.rs:26)). This is a small but
important example of testing failure modes users actually encounter in scripts and pipelines.

### 1.5 Desktop/web separation and portability

Layering is explicit: lower crates do not depend on egui, eframe, winit, or rfd, and the dependency
graph is checked by [`cargo xtask layers`](../../xtask/src/main.rs:20)-[`layers.rs`](../../xtask/src/layers.rs:155).
The architecture places the reusable document/layout/engine core below the UI and app shells
([`docs/architecture.md`](../../docs/architecture.md:9)-[`docs/architecture.md`](../../docs/architecture.md:34)).

The desktop manifest keeps platform dependencies behind target conditions, including macOS, Unix,
Windows, and non-WASM sections ([`apps/wordcraft/Cargo.toml`](../../apps/wordcraft/Cargo.toml:31)-[`apps/wordcraft/Cargo.toml`](../../apps/wordcraft/Cargo.toml:80)).
The web app is a separate binary that uses the same UI/core but compiles browser-only code under
[`cfg(target_arch = "wasm32")`](../../apps/wordcraft-web/src/main.rs:12); its documented differences are
browser file pickers/downloads and no TCP control channel ([`apps/wordcraft-web/src/main.rs`](../../apps/wordcraft-web/src/main.rs:1)-[`apps/wordcraft-web/src/main.rs`](../../apps/wordcraft-web/src/main.rs:9)).
The CI WASM step checks the designated portable crates ([`xtask/src/main.rs`](../../xtask/src/main.rs:119)-[`xtask/src/main.rs`](../../xtask/src/main.rs:149)).

## 2. What protects a writing application from data loss

### High-value practices to retain

* **Transactional edits.** Snapshot before a mutating command, validate parameters, and restore on
  error. This prevents a bad screenplay command from leaving a half-applied scene, selection, or
  undo state. WordCraft's implementation is a concrete reference in [`Session::run()`](../../crates/engine/src/session.rs:1156)-[`Session::run()`](../../crates/engine/src/session.rs:1249).
* **No-panic production paths.** A panic can terminate the process before autosave or recovery runs.
  Deny-level linting catches accidental panic APIs early; `catch_unwind` is a final containment layer,
  not a substitute for validation ([`AGENTS.md`](../../AGENTS.md:9)-[`AGENTS.md`](../../AGENTS.md:16)).
* **Hostile-input testing.** JSON commands, imported project files, persisted settings, and layout
  dimensions can all be malformed or unexpectedly large. Fuzz/property tests turn “must not crash or
  hang” into an executable contract, as shown by [`hostile_params_never_panic()`](../../crates/engine/src/tests.rs:1856).
* **Undo/recovery invariants.** Test that a failed command leaves text and history unchanged, that
  undo restores exact content and caret semantics, and that autosave/reopen does not discard the last
  good version. WordCraft's model round trip and failed-command tests illustrate the first two
  ([`json_round_trip()`](../../crates/doc/src/tests.rs:104), [`failed_command_leaves_document_unchanged()`](../../crates/engine/src/tests.rs:1845)).
* **Deterministic pagination fixtures.** A screenplay is judged by page breaks and scene layout. In
  code-built fixtures, assert stable invariants—page count, element order, margins, and no overlap—rather
  than pixel snapshots everywhere. WordCraft's page tests demonstrate this style
  ([`tests_pages.rs`](../../crates/layout/src/tests_pages.rs:36)).
* **Headless command tests.** They exercise the same editing core without window timing, focus, or
  clipboard nondeterminism. This makes every save/undo/scene-transition regression cheap to run.
* **Layered portability.** Keep the screenplay model and persistence independent of desktop UI APIs;
  portability then becomes a compile-time boundary and a small number of target checks, rather than a
  rewrite when web support arrives.

### Useful but secondary safeguards

Per-command metadata, centralized enablement, and a status error are valuable if automation or a
command palette is planned. File-root confinement and closed-pipe CLI tests become important once the
app has scripting or untrusted imports. Asset attribution and clean-room rules are important project
governance, but they do not directly prevent a user's screenplay from being lost.

## 3. Recommended test plan for an MVP screenplay app

The following is a deliberately smaller stack. It assumes a screenplay model with scenes, action,
character, dialogue, parenthetical, transition, notes, and page settings.

### Layer A — model invariants (many, fast)

Test pure model operations without a UI:

* a new project always has one valid scene/document root;
* element insertion/deletion preserves ordering and valid IDs;
* splitting, joining, and moving blocks preserve all text and metadata;
* caret and selection positions clamp safely after edits;
* undo/redo is exact, including selection and dirty state;
* serialization round trips preserve screenplay structure, text, IDs, and settings;
* hostile lengths, invalid enum strings, missing fields, invalid UTF-8 boundaries, and huge numbers
  return errors or bounded results rather than panicking.

Use ordinary example tests for product rules and a small property-test suite for edit sequences and
serialization. A useful property is: generate a bounded sequence of valid edits, serialize and reload,
then compare normalized model state. Do not begin with a generalized fuzzing farm; first make the
invariants and error types explicit.

### Layer B — command/session contract (many, fast)

Route all mutations through a small command dispatcher. Test every command for:

* valid execution and expected result;
* missing, wrong-type, out-of-range, and oversized parameters;
* disabled-state behavior;
* failed-command rollback of model, selection, undo, redo, and dirty state;
* typing coalescing and distinct undo steps for automatic scene-element transitions;
* unknown command behavior;
* no dialog or filesystem requirement in automation mode.

Add one hostile-parameter sweep over the command registry, analogous to
[`hostile_params_never_panic()`](../../crates/engine/src/tests.rs:1856), but keep external-I/O commands
in a separate integration suite. Add a regression test for every discovered crash or data-loss bug.

### Layer C — pagination fixtures (moderate, deterministic)

Build a small fixture catalog in code or checked-in neutral test data:

* title page and first-page screenplay margins;
* scene heading, action, character, parenthetical, dialogue, and transition blocks;
* long dialogue splitting across pages;
* scene heading orphan/keep rules;
* explicit page breaks and blank pages where the product permits them;
* very long unbroken words and oversized blocks;
* notes or revision markers if included in the MVP;
* caret-to-page mapping and hit testing.

Assert page count, block order, element bounds, usable margins, and continuation markers. Add a few
rendered screenshot or PDF golden tests only after layout geometry is stable; geometry assertions are
less brittle across operating systems and fonts.

### Layer D — persistence and recovery (moderate, high value)

Test the actual save protocol, not only serialization:

* write to a temporary directory and reopen;
* save a new version without destroying the last good version;
* simulate partial writes, rename failures, permission errors, full disks where practical, and corrupt
  files;
* verify recovery chooses the newest complete version and clearly reports stale/corrupt candidates;
* verify autosave does not overwrite a user-selected destination unexpectedly;
* verify crash/restart recovery preserves text, screenplay metadata, and dirty status semantics;
* bound allocation and parsing for hostile project files.

Use a filesystem abstraction or injected test store so failures are deterministic. Keep atomic replace,
backup naming, and recovery policy in one module. These tests are more important to writers than broad
format compatibility.

### Layer E — UI smoke and portability (few, slower)

Test only the highest-risk real frontend paths:

* launch an empty project and render the editor;
* type text through the actual IME/text-event path;
* press Enter/Backspace and confirm the screenplay element transition;
* save, reopen, and confirm the visible text;
* trigger an error and confirm the window remains usable;
* resize the viewport and confirm pagination/caret remain coherent;
* run one desktop smoke path and one WASM compile/build check.

Prefer a headless UI harness or deterministic synthetic events. Keep screenshot tests to stable
landmarks or a small number of golden screens; do not make every UI unit test depend on GPU pixels.

## 4. Practices that are unnecessary complexity for an MVP

### Defer unless the product requires them

* **Word parity catalog and rising parity floor.** The catalog is a useful planning metric for a Word
  clone, but a screenplay app needs acceptance criteria for its own element grammar, not hundreds of
  unrelated Word features. The repository's parity generator is documented at
  [`AGENTS.md`](../../AGENTS.md:43)-[`AGENTS.md`](../../AGENTS.md:45) and implemented in
  [`xtask/parity.rs`](../../xtask/src/parity.rs:1).
* **MCP and remote control channels.** Headless command tests are useful; a network protocol, control
  keys, path roots, and remote UI backend add security and maintenance work. Add them when scripting,
  agents, or integration testing is a committed requirement.
* **Multiple legacy import/export formats.** DOC, DOCX, RTF, ODT, LaTeX, and compatibility fixtures
  enlarge the hostile-input surface. Start with one versioned native format and plain-text export,
  then add formats with their own bounded parser and corpus tests.
* **Full Word typing parity.** The repository's [`tests_typing.rs`](../../crates/engine/src/tests_typing.rs:1)
  is justified by its target. An MVP needs tests for screenplay-specific Enter transitions,
  indentation, tab behavior, paste, and undo—not AutoCorrect, lists, bidi, tracked changes, or math.
* **All-platform native integrations.** macOS events, Windows settings, Unix clipboard variants,
  printing, system fonts, and accessibility should be isolated adapters, not prerequisites for the
  first editing core. Compile checks and one supported desktop platform are enough initially.
* **Repository-wide `xtask ci`.** Formatting, Clippy, tests, and a target compile are worthwhile; an
  asset auditor, layer graph, parity generator, corpus downloader, and release packaging orchestrator
  can be added when the project has enough crates to justify them.

### Risks of copying the larger machinery

1. **Feature drag:** a screenplay team may spend time satisfying Word-shaped metadata and parity
   floors instead of making scene editing reliable.
2. **False confidence:** a large test count can hide missing end-to-end save/recovery tests. Counts are
   not coverage of the user's most damaging failure mode.
3. **Brittle fixtures:** copying Word-observed pixel or exact-pagination expectations introduces font,
   platform, and renderer variance before the screenplay rules are stable.
4. **Over-centralization:** a giant command registry and session snapshot can become difficult to reason
   about. Keep snapshots explicit and small, and separate external side effects from transactional
   model edits.
5. **Slow feedback:** broad format corpora, multiple targets, UI rendering, and property tests can
   make every change expensive. Run fast model/command suites on every change; schedule heavier
   persistence, UI, and portability suites in CI tiers.
6. **Portability by dependency accumulation:** copying desktop/web adapters wholesale can pull platform
   libraries into the core. Enforce the dependency direction from the start, but implement only the
   supported targets.

## 5. Recommended MVP quality gate

For each change, require formatting/linting, unit tests, command hostile-input tests, model round trips,
and the relevant pagination fixture. Before release, add persistence/recovery integration tests, a
desktop UI smoke run, and a WASM compile check if web is a promised target. Treat any production panic,
silent failed save, or unrecoverable edit as a release-blocking defect. Everything else—parity scores,
large format corpora, agent protocols, and broad platform matrices—should be justified by a concrete
screenplay-app requirement rather than inherited from WordCraft's scope.

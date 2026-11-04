# Build Hygiene and Cleanup — 2026-11-04

## Status

**Closed: formatting, check, build, Clippy, and the consolidated four-test
workspace acceptance suite all pass.**

The user explicitly authorized a coding agent to run the build-hygiene checks for
this initiative. The first pass was evidence-only: it made no source or
documentation edits. Git status and diff-stat output were empty after the checks;
only ignored Cargo build artifacts may have been refreshed.

## Initial command results

| Check | Command | Exit | Outcome |
|---|---|---:|---|
| Formatting | `cargo fmt --all -- --check` | 1 | Failed: rustfmt reported 462 formatting-diff hunks across application, core, and test sources. These were formatting differences, not compiler errors. |
| Type check | `cargo check --workspace` | 0 | Passed with 19 core warnings and 9 application warnings. |
| Build | `cargo build --workspace` | 0 | Passed with the same 19 core warnings and 9 application warnings. |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | 101 | Failed because warnings were promoted to errors. The final output reported 52 errors in the core library-test target. |
| Tests | `cargo test --workspace` | 101 | Failed while compiling test targets. No tests executed and no assertion failures were produced. |

The successful check and build do not establish warning-free compilation,
Clippy cleanliness, or test passage.

## Compiler warnings

### Core

- Unused `PrintVisibility` import in [`layout.rs`](../../crates/scriptcraft-core/src/layout.rs:11).
- Deprecated atomic `fetch_update` calls in [`editing.rs`](../../crates/scriptcraft-core/src/editing.rs:110), including lines 110, 1150, 1157, and 1171; [`preservation.rs`](../../crates/scriptcraft-core/src/preservation.rs:44), including lines 44 and 53; and [`lib.rs`](../../crates/scriptcraft-core/src/lib.rs:512).
- Unnecessary mutable binding in [`fdx_storage.rs`](../../crates/scriptcraft-core/src/fdx_storage.rs:507).
- Dead-code warnings for [`corrected_cast_list_candidate_for_scenes()`](../../crates/scriptcraft-core/src/cast_list.rs:86), [`pending_derived_invalidation()`](../../crates/scriptcraft-core/src/editing.rs:435), two save-diagnostic variants in [`fdx.rs`](../../crates/scriptcraft-core/src/fdx.rs:178), [`save_session_with_codec()`](../../crates/scriptcraft-core/src/fdx_storage.rs:440), and unused scene-metadata functions in [`scene_metadata.rs`](../../crates/scriptcraft-core/src/scene_metadata.rs:38).

### Desktop application

- Future-incompatible floating-point literal fallback in [`main.rs`](../../apps/scriptcraft/src/main.rs:1348).
- Unused variables in [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:1258) and [`main.rs`](../../apps/scriptcraft/src/main.rs:1497).
- Dead benchmark operation variants and [`reset_benchmark()`](../../apps/scriptcraft/src/main.rs:339), unused presentation-platform variants and [`viewport_center_anchor_target()`](../../apps/scriptcraft/src/main.rs:2217), and unused compact-status items in [`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:6).

## Clippy diagnostics

With warnings denied, Clippy promoted the compiler warnings above and reported
these additional lint sites:

- [`cast_list.rs`](../../crates/scriptcraft-core/src/cast_list.rs:175): needless borrow.
- [`editing.rs`](../../crates/scriptcraft-core/src/editing.rs:423): `filter().next()` at line 423, unnecessary `map_or` at line 1144, needless explicit lifetime at line 1337, boolean assert comparison at line 2279, and needless generic-argument borrows at lines 2465 and 2511.
- [`fdx.rs`](../../crates/scriptcraft-core/src/fdx.rs:737): large `Result` error at lines 737 and 745, unnecessary `map_or` at line 778, needless borrow at line 829, `expect_used` at line 908, boolean-then `filter_map` at line 1161, redundant closure at line 1458, identical `if` branches at lines 1855 and 2017, useless `into_iter` at line 2343, collapsible match at line 3519, field reassignment after `Default` at lines 3823 and 3874, needless borrow at line 4831, type complexity at line 5030, and cloned reference-to-slice at line 5670.
- [`formatting.rs`](../../crates/scriptcraft-core/src/formatting.rs:55): manual `is_multiple_of` implementation.
- [`layout.rs`](../../crates/scriptcraft-core/src/layout.rs:287): too many arguments at lines 287 and 1613, needless `Option::as_deref_mut` at line 561, unnecessary `map_or` at lines 909 and 1356, and mismatched bit-width type at line 2787.
- [`preservation.rs`](../../crates/scriptcraft-core/src/preservation.rs:561): large enum variant at line 561, too many arguments at line 900, and `expect_err` diagnostics at lines 3314, 3344, 3366, and 3384.
- [`scene_metadata.rs`](../../crates/scriptcraft-core/src/scene_metadata.rs:377): type complexity at line 377, unnecessary `map_or` at line 383, identical `if` branches at line 773, and collapsible `if` at line 900.

## Test-target compilation blocker

[`v4_wo001_empty_managed_paragraphs.rs`](../../crates/scriptcraft-core/tests/v4_wo001_empty_managed_paragraphs.rs:289)
passes `String::new()` where the preserved text-node variant requires `SharedText`.
The compiler reported `E0308` and suggested conversion with `.into()`.

This is a test-compilation failure, not a failing test assertion. The test command
also emitted the existing core warnings and four application-test warnings: the
floating-point fallback and unused variables listed above, plus an unused
`expected_project` in [`main.rs`](../../apps/scriptcraft/src/main.rs:4140).

## Repair constraints

1. Apply standard workspace formatting.
2. Fix the test-target type mismatch without changing its intended assertion.
3. Resolve compiler and Clippy diagnostics with minimal, behavior-preserving edits.
4. Prefer simple idiom corrections and removal of genuinely unused code. Where a
   lint conflicts with an intentional public or architectural boundary, use a
   narrowly scoped and justified allowance rather than a broad crate-level waiver
   or risky redesign.
5. Preserve screenplay semantics, FDX preservation, save safety, pagination,
   exports, and desktop behavior; this is build hygiene, not a feature initiative.
6. Re-run all five commands independently and record exact final exit statuses,
   executed test counts, residual diagnostics, and changed files.

## Hygiene repairs applied

A coding agent applied standard workspace formatting and conservative repairs,
then ran the complete inventory again. The repairs:

- converted the two preserved shared-text/shared-vector test values required by
  the current public types;
- replaced deprecated atomic updates and cleared unused imports, bindings, and
  genuinely unused private helpers;
- applied idiomatic iterator, option, assertion, borrow, initialization, and
  integer-width corrections; and
- retained intentional architectural interfaces through narrowly scoped lint
  allowances rather than broad warning suppression or save/renderer redesign.

Scoped allowances preserve large save-writer diagnostic errors, explicit
renderer/layout/recursive-collector arguments, the inline public preserved-node
representation, and reserved diagnostic number 28. These allowances are located
in [`fdx.rs`](../../crates/scriptcraft-core/src/fdx.rs:733),
[`layout.rs`](../../crates/scriptcraft-core/src/layout.rs:289),
[`preservation.rs`](../../crates/scriptcraft-core/src/preservation.rs:570),
[`main.rs`](../../apps/scriptcraft/src/main.rs:1259), and
[`status_bar.rs`](../../apps/scriptcraft/src/status_bar.rs:719).

The agent reported 46 modified tracked source/test files, with 4,809 insertions
and 2,727 deletions, predominantly standard formatting. `git diff --check`
exited 0. Neither durable Forge documents nor this planning report were edited
by that agent.

## Post-repair command results

| Check | Command | Exit | Outcome |
|---|---|---:|---|
| Formatting | `cargo fmt --all -- --check` | 0 | Passed. |
| Type check | `cargo check --workspace` | 0 | Passed with no warnings reported. |
| Build | `cargo build --workspace` | 0 | Passed with no warnings reported. |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | 0 | Passed with no diagnostics reported. |
| Tests | `cargo test --workspace` | 101 | Test targets compiled and ran; application assertions failed. |

The application test binary ran 204 tests: **190 passed, 14 failed, 0 ignored**.
A serial rerun produced the same counts, ruling out simple test-thread
contention for those failures. Reported groups include rendering/page and scroll
measurements, text casing and role expectations, an unsupported-arc fixture
import, and theme/OS presentation assertions.

A supplemental `cargo test -p scriptcraft-core` invocation exited 101. Its
library test binary ran 234 tests: **204 passed, 30 failed**. Reported groups
include cast-list/scheduler and scoped-edit assertions, FDX/layout round trips,
save/storage/preservation assertions, scene-metadata reconciliation, and
text-selection offsets.

These are behavioral assertion failures, not residual compiler or Clippy
diagnostics. Because the initial workspace test command could not compile its
test targets, this evidence cannot establish whether the assertion failures
predated the hygiene repairs. The repair agent did not weaken assertions or
undertake product refactors to force passage.

## Test-suite consolidation

After the old suite exposed behavioral failures, the user directed that all
existing tests be considered unnecessary and obsolete by default. The cleanup
therefore stopped treating historical work-order tests as release gates and
classified them for retirement, retaining only a small set of consolidated
contract-focused scenarios. The old failures were **not repaired or rerun**;
the suite containing them was retired under that explicit direction.

The classification retired 37 integration files: four were mined for useful
scenarios, 11 were redundant after consolidation, and 22 were obsolete
work-order or implementation-detail coverage. It also retired all 18 inline
unit-test modules—five desktop and 13 core—including desktop rendering,
performance, and implementation oracles. The now-unreferenced legacy JSON
fixture was removed; no source documents under `.reference` were changed.
Initial sections above name some of these retired files as historical evidence.

The sole remaining test target is
[`core_acceptance.rs`](../../crates/scriptcraft-core/tests/core_acceptance.rs:1),
with four headless scenarios:

1. [`semantic_editing_preserves_roles_unicode_history_and_dirty_state()`](../../crates/scriptcraft-core/tests/core_acceptance.rs:45)
   covers explicit role transitions, UTF-8 editing, undo/redo, and dirty state.
2. [`headless_layout_and_noncanonical_exports_are_repeatable_and_non_mutating()`](../../crates/scriptcraft-core/tests/core_acceptance.rs:73)
   covers repeatable layout, PDF/plain-text boundaries, loss disclosure, and
   export non-mutation.
3. [`native_fdx_reopen_preserves_semantics_opaque_source_and_established_ids()`](../../crates/scriptcraft-core/tests/core_acceptance.rs:94)
   covers native semantic/source preservation and established paragraph IDs.
4. [`failed_save_as_keeps_existing_destination_and_live_unsaved_session()`](../../crates/scriptcraft-core/tests/core_acceptance.rs:132)
   covers existing-destination refusal without changing destination bytes,
   session content, dirty state, or save-target binding.

The optional scene-derived arc scenario was omitted rather than preserving a
large implementation-specific fixture apparatus. No desktop automated tests or
inline source test modules remain.

## Final verification

| Check | Command | Exit | Outcome |
|---|---|---:|---|
| Formatting | `cargo fmt --all -- --check` | 0 | Passed. |
| Type check | `cargo check --workspace` | 0 | Passed. |
| Build | `cargo build --workspace` | 0 | Passed. |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | 0 | Passed. |
| Tests | `cargo test --workspace` | 0 | Four consolidated integration tests passed; zero failed. |

The final test inventory contains zero application unit tests, zero core unit
tests, four passing integration tests, and zero doc tests. `git diff --check`
also exited 0, and source scans found no remaining inline test modules or test
configuration markers in application/core sources.

The coding agent reported 57 tracked files changed with 1,769 insertions and
30,585 deletions, plus the new untracked consolidated acceptance file. The large
deletion count is intentional test retirement. No tracked planning, Forge, or
reference-source files were changed by the coding agents; this report and the
durable documentation updates are authored separately during closeout.

## Closeout boundaries

- The retired suite's 44 assertion failures are historical evidence, not claims
  that those product behaviors were fixed.
- Four passing acceptance tests establish only their bounded scenarios; they do
  not replace desktop manual validation or prove every FDX/editing contract.
- No desktop window/manual run was performed in this cleanup.
- Build, warning-free Clippy, formatting, and the consolidated workspace test
  suite are clean at the recorded final run.

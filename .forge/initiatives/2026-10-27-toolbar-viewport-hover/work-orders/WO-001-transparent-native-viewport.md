# Work Order: WO-001 — Transparent Native Viewport

## Metadata

- **Status:** `complete`
- **Assignee:** Desktop host implementer (fresh session)
- **Phase/order:** 1 of 3
- **Initiative:** [`../toolbar-viewport-hover-v1.md`](../toolbar-viewport-hover-v1.md:1)
- **Validation:** [`../validations/validation-WO-001.md`](../validations/validation-WO-001.md)
- **Implementer session/agent ID:** `code-implementer-toolbar-viewport-hover-2026-10-27T0535Z`
- **Session exception:** none

## Summary

Configure the native eframe viewport with hidden title text, transparent titlebar, and full-size content view using the requested `ViewportBuilder` flags, while preserving application startup and editor behavior.

## In scope

- Locate the eframe native runner/options construction in [`main()`](../../../../apps/scriptcraft/src/main.rs:2780).
- Apply the equivalent of `with_title_hidden(true)`, `with_titlebar_transparent(true)`, and `with_fullsize_content_view(true)` using the current eframe API.
- Add a compile/testable options assertion where possible and document platform limitations.

## Out of scope

- Toolbar hover, width allocation, color source, core, persistence, exports, and editor layout.

## Requirements and blueprints

- **Requirements:** Verify REQ-WRITING-LOOP-005 unchanged; this is desktop host presentation.
- **Blueprints:** Update desktop viewport presentation in [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:1).

## Validation

- **Automated:** format, check, tests, build, Clippy with `-D warnings`, and diff check.
- **Acceptance:** The native runner uses the requested viewport flags where supported; startup remains valid and unsupported-platform behavior is documented honestly.

## Durable-document impact

- **Requirements impact:** `verify unchanged`
- **Blueprint impact:** `update`
- **Rationale:** Native viewport/window presentation changes without changing product semantics.

## Implementation evidence

- **Changed files:**
  - [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1461) — added the shared `native_options()` construction, configured the installed eframe 0.29 equivalents `with_title_shown(false)`, `with_titlebar_shown(false)`, and `with_fullsize_content_view(true)`, and added a compile/testable options assertion.
  - [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:90) — documented native viewport ownership, the installed API mapping, and platform limitations.
  - [`WO-001-transparent-native-viewport.md`](WO-001-transparent-native-viewport.md:1) — recorded implementation evidence, exact gate results, and status.
  - [`execution-log.md`](../execution-log.md:35) — appended dispatch/completion evidence.
- **Requirements verification:** REQ-WRITING-LOOP-005 was checked and remains unchanged; this host presentation change does not alter toolbar commands, semantic editing, persistence, exports, or layout behavior.
- **Options coverage:** `native_options_configure_the_requested_viewport_presentation` asserts the three resulting `ViewportBuilder` fields without opening a native window.
- **Platform limitation:** eframe 0.29 documents the title visibility, titlebar visibility, and full-size content settings as macOS-specific; other platforms may ignore them or retain platform-managed decorations. No native visual smoke result is claimed because the requested native smoke was unavailable in this implementation session.

## Required gate results

The following project-policy gates were run after implementation; exact results are recorded here for fresh validation to assess:

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | `PASS` — exit 0 |
| `cargo check --workspace` | `PASS` — exit 0 |
| `cargo test --workspace` | `PASS` — exit 0; 136 tests passed, 0 failed, 0 ignored across the workspace, including `native_options_configure_the_requested_viewport_presentation` |
| `cargo build --workspace` | `PASS` — exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | `PASS` — exit 0 |
| `git diff --check` | `PASS` — exit 0; no whitespace errors |

- **Comparable diff:** `git status --short` reported modified [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs:1461) and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:90), plus the initiative evidence directory as untracked; `git diff --stat` reported 2 tracked files with 36 insertions and 5 deletions, and `git diff --check` exited 0. The complete scoped file list, including the untracked work-order and log records, is listed above.
- **Follow-up:** Fresh validation remains required; WO-002 and WO-003 were not implemented.

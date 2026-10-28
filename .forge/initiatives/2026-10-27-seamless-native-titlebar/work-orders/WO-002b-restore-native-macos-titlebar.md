# Work Order: WO-002b — Restore Native macOS Titlebar

## Metadata

- **Status:** `in-progress` — directly commissioned by the user after the WO-002a visual gate failed.
- **Initiative:** [`../seamless-native-titlebar-v1.md`](../seamless-native-titlebar-v1.md)
- **Original failed order:** [`WO-002-native-titlebar-double-click.md`](WO-002-native-titlebar-double-click.md) — remains `blocked`; its validation remains `fail`.
- **Unsuccessful attempted fix:** [`WO-002a-native-transition-scaling-mitigation.md`](WO-002a-native-transition-scaling-mitigation.md) — remains `blocked`; its one renewed easing attempt did not resolve the user's visual popping report.
- **Authorization:** User direction, 2026-10-28: “Visual popping remains; stop the WO-002a attempt and keep WO-002 blocked/failed. Lets commission an agent to restore the mac-native approach, keep the OS native header bar, and adjust our toolbar size accordingly.”
- **Implementation session:** This separately commissioned Code implementation; session ID is not exposed by the execution interface.
- **Validation:** No fresh validator is dispatched by this work order before the user's implementation-time visual verification gate.

## Implementation evidence and current gate — 2026-10-28

- **Source changes:** `apps/scriptcraft/src/main.rs` now requests visible native title/header and buttons, decorated/opaque window presentation, and `fullsize_content_view(false)`. Native content no longer allocates a synthetic 28-point titlebar band or app-side `performZoom:` content handler; toolbar layout starts at the content panel origin with its prior width and height. The WO-002a page-scale easing and its dedicated test were reverted. Custom-titlebar/transition machinery and unrelated tests/assertions were retained.
- **Focused tests:** `native_startup_selects_the_single_locked_viewport_tuple` and `toolbar_starts_at_content_boundary_and_preserves_full_width_and_height` each passed (1 passed, 0 failed).
- **Clean workspace checks:** `cargo clean` removed 4,694 files / 1,012.8 MiB. `cargo fmt --all -- --check`, both focused tests, `cargo check --workspace`, `cargo test --workspace`, `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check` all passed. Workspace tests: app 89, core unit 45, integration 37, doc tests 0; 171 passed, 0 failed.
- **Changed paths:** `.forge/initiatives/2026-10-27-seamless-native-titlebar/execution-log.md`; `.forge/initiatives/2026-10-27-seamless-native-titlebar/seamless-native-titlebar-v1.md`; `.forge/initiatives/2026-10-27-seamless-native-titlebar/work-orders/WO-002a-native-transition-scaling-mitigation.md`; `.forge/initiatives/2026-10-27-seamless-native-titlebar/work-orders/WO-002b-restore-native-macos-titlebar.md` (new); `apps/scriptcraft/src/main.rs`. WO-002, `validation-WO-002.md`, and WO-003 were not changed. No stage or commit.
- **Clean-build identity:** macOS 26.6.2 (25G83), arm64; Rust 1.99.0 (`b940084d7eb6a299eb4bfeb8e34901bc051e7ac4`), Cargo 1.99.0 (`5f94df478`); `target/debug/scriptcraft`, 21,719,248 bytes, mtime `2026-10-28T01:20:09-0500`; SHA-256 `34c2d3c43d3bccfcaec26e7db8f6c60117d7b50f30316086a79efde9a522ee27`; Mach-O UUID `7A32BCB8-E812-32BA-B2C6-4E99134BB31C` (arm64).
- **Launch/user gate:** No stale `target/debug/scriptcraft` process was present. The clean-built binary was launched as PID `25959` (`nohup target/debug/scriptcraft`, log `/tmp/scriptcraft-wo002b.log`). The user must visually verify the actual normal native header, toolbar placement/controls, preserved app/document behavior, and OS-owned titlebar double-click/zoom. No visual result is claimed. WO-002 remains `blocked` / `fail`; WO-002a remains `blocked`; neither is complete or validated. Stop here until the user gate passes. Only then may a separate agent draft WO-003 for endeavor-bloat cleanup, followed by the separate WO-003 draft approval gate.

## Objective

Restore the normal macOS window presentation: show the OS-owned native title/header bar, keep application content below it rather than extending under it, and place the existing application toolbar at the top of the ordinary content area without a synthetic titlebar-sized reservation. Leave native titlebar double-click/zoom behavior to macOS. Revert the unsuccessful WO-002a document page-scale easing.

## Scope and preservation

1. In the selected `NativeMacOs` startup path, use the ordinary decorated native window/titlebar presentation; do not hide the native title, make the titlebar transparent, or request full-size content behind it.
2. Remove the native-mode synthetic 28-point top inset and its blank content-band double-click dispatch. The content view's actual top boundary is already below the OS-owned titlebar; the toolbar starts at that boundary.
3. Preserve the toolbar's existing controls, allocation, behavior, and visual styling; preserve application behavior and native document content. Do not redesign toolbar sizing/controls or page layout beyond removing the synthetic inset and obsolete test expectation.
4. Restore operating-system ownership of titlebar double-click/zoom. Do not issue app-side viewport maximize/zoom commands from content or titlebar code in the selected native mode.
5. Revert the WO-002a `Context::animate_value_with_time` native page-scale easing and its easing-specific focused test. Restore the existing immediate page-fit metrics; do not replace it with another scale policy.
6. Keep custom-titlebar-mode and transition-policy implementation and unrelated tests/assertions intact. No broad cleanup, random assertion removal, dependency changes, requirements/blueprint edits, or WO-003 creation/editing.

## Required verification

- Add/update focused tests that assert the selected native options retain normal visible titlebar/decorations and do not use transparent-titlebar/full-size content flags, and that native toolbar layout begins at the content panel top with no 28-point reservation while preserving toolbar height/width.
- Run `cargo fmt --all -- --check`, focused app tests, `cargo check --workspace`, `cargo test --workspace`, `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check`. Record outcomes and exact changed paths.
- Clean-build and launch the macOS app, stop any stale prior instance if needed, and record the binary identity. Return the launched build to the user for direct visual verification of the visible native header, toolbar placement/controls, preserved app/document behavior, and OS-owned titlebar double-click/zoom.
- Do not claim visual success, self-validate, dispatch a separate validator, or mark WO-002/WO-002a complete. Stop for the user's actual verification/approval gate.
- Only after the user explicitly passes this restoration gate may a separate agent draft WO-003 to remove endeavor bloat. Then stop for approval of that WO-003 draft before any cleanup implementation.

## Changed-file boundary

Expected implementation paths are `apps/scriptcraft/src/main.rs` and this work order, the initiative plan, and execution log. Do not touch WO-002, its validation, WO-003, requirements, blueprints, or unrelated source/tests.

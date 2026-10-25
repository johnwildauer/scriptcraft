# Validation: WO-009 — Unified Live Editing Surface

## Metadata

- **Validation status:** `fail`
- **Validator:** Independent fresh-context validator
- **Session/agent ID:** `validation-session-2026-10-25-wo-009`
- **Date:** 2026-10-25
- **Work order:** [`WO-009-unified-live-editing-surface.md`](../work-orders/WO-009-unified-live-editing-surface.md)
- **Implementation source:** [`main.rs`](../../../apps/scriptcraft/src/main.rs)
- **Validation process:** [`05-validation.md`](../../../process/05-validation.md)
- **Product source edited:** No

## Comparison basis

Read the validation process, WO-009, the scope-correction note, the UI-correction design review, the execution log, and the relevant core APIs. The implementation comparison was made against the current [`main.rs`](../../../apps/scriptcraft/src/main.rs), [`editing.rs`](../../../crates/scriptcraft-core/src/editing.rs), and [`layout.rs`](../../../crates/scriptcraft-core/src/layout.rs), plus the recorded implementation evidence. The repository has no committed baseline; validation therefore used current source, focused tests, and the reported changed-file record.

## Checks and evidence

### Surface structure

- **One primary live page-layout surface:** PASS. [`live_document_surface()`](../../../apps/scriptcraft/src/main.rs:271) is the only central document renderer and calls [`paint_page()`](../../../apps/scriptcraft/src/main.rs:169) for page sheets inside one scroll area. The toolbar identifies the page-layout surface and has no Continuous/Page mode control.
- **No duplicate per-element `TextEdit` or editor buffer:** PASS. A source scan found no `TextEdit`, `continuous_view`, `page_view`, `text_buffer`, or second editor state in [`main.rs`](../../../apps/scriptcraft/src/main.rs). The app state stores [`EditingSession`](../../../apps/scriptcraft/src/main.rs:13), focus/status/error/path state only.
- **No Continuous/Page editing split:** PASS. The only document call is [`live_document_surface()`](../../../apps/scriptcraft/src/main.rs:466); page layout is the default and only editing presentation.

### Core-routed editing behavior

- **Source-mapped caret placement and hit testing:** PASS. Pointer coordinates are translated by [`caret_from_pointer()`](../../../apps/scriptcraft/src/main.rs:127) to [`ScreenplayLayout::hit_test()`](../../../crates/scriptcraft-core/src/layout.rs:122), and caret painting uses [`ScreenplayLayout::caret_rect()`](../../../crates/scriptcraft-core/src/layout.rs:143). The focused app test [`caret_selection_mapping_is_source_mapped()`](../../../apps/scriptcraft/src/main.rs:566) passed.
- **Direct typing/replacement, Delete, Backspace, Enter, horizontal movement:** PASS by source and focused tests. [`handle_surface_input()`](../../../apps/scriptcraft/src/main.rs:290) routes text and keys to [`EditingSession::insert()`](../../../crates/scriptcraft-core/src/editing.rs:132), [`delete()`](../../../crates/scriptcraft-core/src/editing.rs:143), [`backspace()`](../../../crates/scriptcraft-core/src/editing.rs:170), [`enter()`](../../../crates/scriptcraft-core/src/editing.rs:216), and [`move_horizontal()`](../../../apps/scriptcraft/src/main.rs:341). The app replacement/delete/backspace and role-aware Enter tests passed.
- **Role transition:** PASS. Enter delegates to the explicit core transition policy in [`enter()`](../../../crates/scriptcraft-core/src/editing.rs:216); the app test verified Scene Heading → Action.
- **Undo/redo:** PASS by source and existing core behavior. Toolbar and command-key handling delegate to [`EditingSession::undo()`](../../../apps/scriptcraft/src/main.rs:328) and [`redo()`](../../../apps/scriptcraft/src/main.rs:334); the workspace test suite includes undo/redo coverage.
- **Recoverable errors:** PASS by source. Mutations flow through [`mutate()`](../../../apps/scriptcraft/src/main.rs:103), which retains the session and surfaces a bounded error through [`show_error()`](../../../apps/scriptcraft/src/main.rs:94). Core failed-edit rollback tests passed.

### Independent command results

All commands completed successfully:

- `cargo fmt --all -- --check` — PASS
- `cargo check --workspace` — PASS
- `cargo build --workspace` — PASS
- `cargo test --workspace -- --nocapture` — PASS: 4 app tests, 22 core unit tests, 3 WO-008 integration tests, and doc-test target with zero failures
- `cargo clippy --workspace --all-targets -- -D warnings` — PASS
- Bounded `cargo run -p scriptcraft` smoke — binary launched (`target/debug/scriptcraft`), then was terminated after five seconds with `bounded_smoke_exit=143`. No visible-window or manual interaction result was claimed; project policy does not configure this as a required manual validation gate.

## Finding

### Blocker — range drag selection loses the drag anchor

**Severity:** blocker; plainly unmet WO-009 acceptance criterion.

In [`paint_page()`](../../../apps/scriptcraft/src/main.rs:169), the pointer path handles both `drag_started()` and `dragged()`, but it creates a caret selection whenever `response.dragged()` is false:

```rust
let selection = if response.dragged() {
    Selection { anchor: self.session.selection().anchor, focus: position }
} else {
    Selection::caret(position)
};
```

The initial pointer-down/drag-start event therefore replaces the pre-drag anchor with a collapsed caret before the subsequent drag events use it. The later [`response.dragged()`](../../../apps/scriptcraft/src/main.rs:185) path consequently anchors from the new pointer-down position rather than preserving the original selection anchor. This does not independently demonstrate a reliable drag/range selection from the initial press, while WO-009 explicitly requires drag/range selection routed through [`EditingSession::set_selection()`](../../../crates/scriptcraft-core/src/editing.rs:126).

The focused tests cover programmatic selection replacement, not the pointer drag-start sequence, so they do not close this blocker.

### Required fix

Create and dispatch the lettered fix work order **WO-009a — preserve pointer drag selection anchor**, followed by a fresh validation. The fix must keep the pointer-down position as a stable drag anchor through the drag gesture, route each updated focus through [`EditingSession::set_selection()`](../../../crates/scriptcraft-core/src/editing.rs:126), and add a focused regression test for a multi-character or cross-element drag range. Do not implement the fix in this validation session.

## Verdict

**FAIL — WO-009 remains `in-review`.** The single-surface architecture, source mapping, core mutation routing, role transition, undo/redo, recoverable-error path, and required Cargo gates align, but reliable drag/range selection is a required writing-loop behavior and is not established by the current pointer implementation. No product code was changed. No other hard-stop finding was identified.


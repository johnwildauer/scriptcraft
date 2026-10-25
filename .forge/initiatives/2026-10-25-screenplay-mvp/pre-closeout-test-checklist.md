# MVP pre-closeout test checklist

Use this checklist for the consolidated human review before closing the initiative or opening fix work.
The automated validation artifacts already cover the headless contracts; these checks focus on observable
desktop behavior, workflow completeness, and gaps that terminal-only validation could not confirm.

## Launch and workspace

- [x] Launch the desktop app from a clean terminal with `cargo run -p scriptcraft`.
- [x] Confirm a window appears, remains responsive, and closes without a panic or data-loss prompt.
- [x] Confirm the initial project contains one editable empty Action element.
- [x] Confirm resizing the window does not crash, lose text, or corrupt the active selection.

## Core writing loop

- [x] Type ordinary ASCII text into the active editor and confirm it appears in the document model/view.
- [x] Type accented text, emoji, combining marks, and non-Latin text; confirm no replacement characters or
  corrupted UTF-8 appear.
- [x] Exercise the scene workflow: Scene Heading → Action, Character → Dialogue, Parenthetical → Dialogue,
  Dialogue → Action/Character, and Transition → Scene Heading.
- [ ] Split an element with Enter and verify text before/after the caret is preserved.
- [ ] Use Backspace at the start of an element and verify it joins with the previous element correctly.
- [ ] Select a range, replace it, delete it, and confirm the selection/caret remains valid.
- [ ] Undo and redo typing, a role transition, a split, a join, and a deletion.
- [ ] Trigger an invalid or unavailable operation and verify a visible recoverable error with no model change.

## Layout and view behavior

- [x] Confirm continuous mode displays all authored elements in order.
- [x] Toggle page preview and confirm page sheets, margins, role indents, and page count are coherent.
- [ ] Enter enough action/dialogue text to force wrapping and multiple pages; verify no overlap or clipped text.
- [ ] Verify scene headings stay with following content when space permits.
- [x] Verify an oversized dialogue/action element splits instead of creating a blank-page loop.
- [x] Toggle between continuous and page preview after edits and confirm content/roles remain identical.
- [ ] Confirm caret/selection behavior remains usable after wrapping, pagination, scrolling, and resize.

## Native project persistence and recovery

- [ ] Save a new project to a chosen path and confirm the file is valid versioned JSON.
- [x] Edit after saving, close/reopen, and verify title metadata, roles, text, stable IDs, and pagination.
- [x] Save As to a new path and verify the original remains unchanged.
- [ ] Simulate or observe a save failure and confirm the previous good file and dirty state are preserved.
- [ ] Create a dirty recovery snapshot, restart, and confirm the newest valid candidate can be identified.
- [ ] Add malformed/unsupported recovery files and confirm they are ignored with a visible safe error/status.
- [ ] Confirm generated runtime files are not accidentally written into the repository root during normal use.

## PDF and plain-text export

- [x] Export a multi-page PDF and open it in a PDF viewer; confirm it has the expected page count and readable
  text/layout.
- [x] Export plain text and confirm the documented loss report identifies lost roles/IDs/layout semantics.
- [ ] Export while dirty and verify export does not mark the native project clean.
- [ ] Trigger an export failure and confirm the editor remains usable and the document is unchanged.

## Safety and repository hygiene

- [ ] Run `cargo fmt --all -- --check`.
- [ ] Run `cargo check --workspace`, `cargo build --workspace`, and `cargo test --workspace`.
- [ ] Run `cargo clippy --workspace --all-targets -- -D warnings`.
- [ ] Confirm `target/`, `.DS_Store`, and other generated artifacts remain ignored.
- [ ] Confirm `git status` shows only intentional review/checklist changes before any follow-up commit.

## Findings policy

Classify findings before changing behavior:

- **Blocker:** data loss, panic, invalid native-file preservation, broken core writing loop, or an acceptance
  criterion that cannot be demonstrated. Create a scoped fix work order before implementation.
- **Warning:** usability limitation or missing visual polish that does not invalidate the MVP contract. Record
  it as a follow-up unless the owner promotes it to a fix.
- **Note:** expected MVP limitation, environment limitation, or improvement opportunity.

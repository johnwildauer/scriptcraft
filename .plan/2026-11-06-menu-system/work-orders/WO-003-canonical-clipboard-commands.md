# WO-003 — Canonical clipboard Edit commands

- **Status:** Implementation submitted; user validation pending
- **Initiative:** [`menu-system-v1.md`](../menu-system-v1.md)
- **Depends on:** [`WO-002`](WO-002-non-clipboard-edit-commands.md)
- **Validation owner:** User; no agent-run checks, reviews, or validators

## Single outcome

Add Cut, Copy, and Paste to the shared dispatcher using canonical screenplay text, explicit clipboard errors, and transactional edit behavior.

## Scope

1. Add a direct desktop dependency on `arboard` in [`apps/scriptcraft/Cargo.toml`](../../../apps/scriptcraft/Cargo.toml), using the compatible version already resolved transitively where practical.
2. Extend [`app_commands.rs`](../../../apps/scriptcraft/src/app_commands.rs) with Cut, Copy, and Paste plus one narrow clipboard adapter for text reads/writes.
3. Extract ordered noncollapsed selections from canonical UTF-8 body elements. Respect selection direction while copying start-to-end, preserve exact casing, and insert one newline per crossed paragraph boundary.
4. Implement Copy without changing the session. Disable or safely reject a collapsed selection.
5. Implement Cut by writing clipboard text first and calling transactional Delete only after clipboard success. Clipboard failure leaves content and selection unchanged.
6. Implement Paste by reading clipboard text on activation and calling [`insert_paste()`](../../../crates/scriptcraft-core/src/editing.rs:625). Empty or nontext clipboard data produces a privacy-safe status/error without semantic mutation.
7. Extend command availability for selection-dependent Cut/Copy and modal-safe Paste. Do not poll clipboard contents every frame merely to determine enablement.
8. Keep clipboard text separate from print-layout plain-text export: no AllCaps transform, HiddenText omission, page layout, or role inference.

## Structural acceptance for user review

- Clipboard reads/writes have one desktop-owned boundary and actionable failure handling.
- Cut never deletes before clipboard success.
- Copy uses canonical editing text rather than rendered/export text.
- Paste remains one existing transactional session edit.

## User-owned behavior review

- Cover Unicode, multi-code-unit characters, reversed selections, within-paragraph and cross-paragraph selections, and empty selections.
- Verify clipboard failure and empty/nontext Paste leave the project unchanged.
- Confirm copied newlines and exact casing, Cut/Undo restoration, and Paste/Undo behavior.
- Confirm no document text or filesystem data appears in diagnostics.

## Handoff requirements

Record the resolved dependency version, canonical newline/casing contract, clipboard error wording, and unverified platform behavior. Do not claim native OS clipboard acceptance without user testing.

## WO-003 implementation handoff — 2026-11-06

- **State and checkout:** Implementation is submitted for user review and validation only; WO-003 is not marked accepted or validated. Work remained on the existing `dev/2026-11-06-menu-system` branch. The pre-existing modified sources and untracked plan/reference work were preserved; no other branch was created, and nothing was stashed, discarded, staged, or committed.
- **Prerequisite integration cleanup:** Corrected the stale `ToolbarBehavior::Action(ToolbarAction)` type reference in [`toolbar_items.rs`](../../../apps/scriptcraft/src/toolbar_items.rs) to use the already imported shared `AppCommand` identity. This is the narrow WO-001 migration remnant reported by WO-002. It was not verified by compilation or another validator.
- **Dependency and exact source changes:** Added direct `arboard = "3.7.0"` to [`apps/scriptcraft/Cargo.toml`](../../../apps/scriptcraft/Cargo.toml) and added `arboard` to the `scriptcraft` package's dependency list in [`Cargo.lock`](../../../Cargo.lock). The existing lockfile recorded arboard 3.7.0 when inspected; no dependency resolver was run, so no new resolution outcome is claimed. [`app_commands.rs`](../../../apps/scriptcraft/src/app_commands.rs) now includes Cut, Copy, and Paste identities; a narrow `DesktopClipboard` adapter for text reads/writes; canonical UTF-8 selection extraction; dispatcher behavior; and clipboard availability fields.
- **Canonical clipboard contract:** A noncollapsed selection is ordered from start to end regardless of anchor direction, sliced from body-element UTF-8 text, preserves exact casing and all selected characters, and inserts exactly one `\n` for each crossed element/paragraph boundary. The extraction does not consult layout, inferred role, AllCaps, HiddenText export omission, or print-export behavior.
- **Clipboard and transaction semantics:** Copy writes the extracted text without touching the session. Cut writes first and calls the existing transactional `session.delete()` only after clipboard success; a failed write reports `Clipboard write failed; screenplay unchanged` and leaves content and selection unchanged. Paste reads only on activation and sends nonempty text to existing `insert_paste()` as one session edit. Empty clipboard text reports `Paste unavailable: clipboard text is empty; screenplay unchanged`; clipboard initialization/read failure, including nontext data, reports `Paste unavailable: clipboard has no readable text; screenplay unchanged`. These static notices disclose no clipboard or document contents. An edit failure after a successful Cut clipboard write follows the existing mutation error path.
- **Availability:** Cut and Copy are enabled only for a noncollapsed selection and when FDX migration is not pending. Paste is enabled whenever migration is not pending; clipboard contents are never polled for frame-by-frame enablement. Activation recomputes the projection, so stale or modal-blocked commands are ignored.
- **Changed paths:** [`apps/scriptcraft/Cargo.toml`](../../../apps/scriptcraft/Cargo.toml), [`Cargo.lock`](../../../Cargo.lock), [`app_commands.rs`](../../../apps/scriptcraft/src/app_commands.rs), [`toolbar_items.rs`](../../../apps/scriptcraft/src/toolbar_items.rs), this order, [`WO-002-non-clipboard-edit-commands.md`](WO-002-non-clipboard-edit-commands.md), and [`menu-system-v1.md`](../menu-system-v1.md). No menu UI, shortcut consolidation, native menu, or later work-order scope was started; screenplay-core was not changed.
- **Evidence limitations and remaining user review:** No build, check, test, lint, formatting command, application launch, manual verification, review, or independent validator was run. UTF-8/reversed/multielement behavior, clipboard failure semantics, native OS clipboard behavior, and Cut/Undo/Paste/Undo remain unverified and require user-owned validation. No native clipboard acceptance or runtime behavior is claimed.

### WO-010 integrated-source reconciliation

The desktop manifest directly declares `arboard = "3.7.0"`; this is the version in the lockfile as
well. `DesktopClipboard` remains the sole explicit read/write boundary. Canonical selection text
retains exact casing and selected UTF-8 with one newline at each crossed element boundary; Cut writes
before deleting, and Paste reads only when activated. These are source contracts, not verified
clipboard outcomes; user review of Unicode, reversed/cross-paragraph selection, failure, and OS
clipboard behavior remains pending.

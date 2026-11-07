# WO-004 — Live cursor and modifier guidance in the status center

- **Submission date:** 2026-11-07
- **Branch:** `dev/2026-11-06-ui-indicators-platform-shell` (existing user-designated branch; no switch)
- **State:** Implementation submitted; user review, validation, and acceptance pending
- **Scope:** WO-004 only. WO-001–WO-003 remain unvalidated starting submissions and are preserved.
- **Validation owner:** User under the authoritative policy in [project-manifest.md](../../../.forge/project-manifest.md).

## Implementation

The center projection is now derived from the current `EditingSession`, editor focus, IME state/current
frame IME events, migration-modal state, current modifiers, and host shortcut platform. Outside the
eligible editor state it yields **Ready**. A collapsed caret must be at the canonical UTF-8 end of its
element (including an empty element) before paragraph guidance is considered. The projection calls
WO-003's `EditingSession::paragraph_key_targets()` and displays only `Target(role)` entries; it neither
reimplements paragraph policy nor simulates edits. Unsupported modifiers suppress paragraph hints.
Exact supported held element chords take priority even over a noncollapsed selection.

Base and extended digit/role rows are projected from the existing `EDITOR_ELEMENT_BINDINGS` table.
The mandated presentation sequence controls row order; role labels come from canonical formatting
metadata and every digit comes from the shared binding table. Extended presentation starts with
Summary without changing key bindings or dispatch order. macOS uses Command for base and
Command+Control for extended;
Windows uses Control for base and Control+Shift for extended. Paragraph guidance includes the matching
`[Cmd]` or `[Ctrl]` hold hint. Releasing/changing modifiers restores the ordinary projection.

Modifier lists are measured against the available center width, use a status-local compact font that
shrinks to a floor, and remain one-line clipped at narrow widths. The accessibility label retains the
full untruncated string. The regular status row geometry, height, page counter, and right controls are
not sized from this content. Modifier changes observed in raw input request repaint directly and do
not create semantic editor commands or menu FIFO entries. During the migration modal the existing
toolbar/menu/control suppression remains; a status footer shows only noninteractive **Ready**, with no
Diagnostics disclosure.

WO-002's Diagnostics source gate remains disabled. Diagnostic messages do not enter the center.
Existing title, menu/accelerator policy, text/IME/key ordering, and Enter/Tab/element command routing
were not intentionally changed.

## Exact changed paths

- `apps/scriptcraft/src/status_bar.rs` — typed live center projection, shared-core target use,
  shared-binding role lists, accessible clipped painting, measured modifier-list sizing, modal Ready
  center, and focused tests.
- `apps/scriptcraft/src/shortcuts.rs` — shared exact modifier predicate for guidance, binding-table
  hint projection in role-label order, and chord/table tests; command mappings are unchanged.
- `apps/scriptcraft/src/main.rs` — current session/input-state projection wiring, modifier-only repaint
  tracking, Ready-only modal status footer, and modifier transition test.
- `.forge/requirements/features/screenplay-writing-loop.md` — durable center priority, target, chord,
  accessibility, clipping, and repaint requirements.
- `.forge/blueprints/technical-context.md` — projection/resolver ownership, modal boundary, modifier
  repaint, and measured/accessible center-list presentation.
- `.forge/project-manifest.md` — dated WO-004 implementation submission and unvalidated evidence.
- `.plan/2026-11-06-ui-indicators-platform-shell/ui-indicators-platform-shell-v1.md` — WO-004 status
  and implementation handoff summary.
- This handoff.

No core source, command catalog, dependency, or lockfile changed. WO-005–WO-007 were not implemented.

## Authored coverage; not executed

- Every role with empty/nonempty end positions, matching core `Target` entries, and omission of
  unavailable/no-op entries; interior and noncollapsed selection suppression; exact held-chord priority
  with selection; focus-loss, IME, migration-modal, and modifier-release behavior.
- All ten base and seven extended role bindings, canonical labels/order, Summary-first extended order,
  both host chord projections, wrong/extra modifiers, and host-correct paragraph hold labels.
- Modifier-only press/release transition detection; status center fitting at wide/narrow dimensions,
  bounded row/right-control geometry, and full text projection independent of clipping.
- Diagnostics remains source-disabled and the Ready projection remains isolated from diagnostic text.

No test, build, check, lint, formatting command, launch, manual verification, review, independent
validator, or other validation was run. Compilation, egui modifier/event delivery on macOS and Windows,
IME frame timing, modifier-only repaint behavior, default/narrow-window list legibility, accessible
full-label exposure, migration-modal footer rendering, and preservation of all existing interactions
remain unverified. The compact font has a lower bound, so the complete visual list may still clip when
the available center region is too narrow; the accessible label remains complete by implementation
intent. No implementation blocker was identified from source inspection; no behavior or acceptance is
claimed.

## User-owned acceptance focus

Review and exercise all 17 role bindings on macOS and Windows, including empty/nonempty end states,
representative interiors, every `[Tab]`/`[Enter]` result, unavailable targets, same-role General Tab,
and Parenthetical outcomes. Hold/release exact base and extended chords; try wrong platform, Option/Alt,
extra Shift/Control, and focus changes. Verify selection, IME preedit/commit/disable events, modal Ready,
and modifier-only repaint. Inspect the complete lists at default and narrow window widths in Day,
Night, and Midnight appearances, and confirm the accessible full text, stable row/right controls,
disabled Diagnostics, unchanged keyboard routing, title/menu behavior, and document safety.

This handoff is not validation evidence or acceptance. WO-005 through WO-007 remain pending.

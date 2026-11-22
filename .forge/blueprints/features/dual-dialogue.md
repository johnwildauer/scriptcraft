# Feature-Area Blueprint: Dual Dialogue

> This feature composes bounded grouping over ordinary body paragraphs with transactional editing, source preservation, transient editor navigation, and a separate Print projection.

## Scope and requirements

- **Requirements:** [Dual dialogue](../../requirements/features/dual-dialogue.md); [Exports and print](../../requirements/features/exports-and-print.md) owns `AC-WRITING-013.7`.
- **Technical context:** [Technical context](../technical-context.md)
- **Requirements traced:** `REQ-WRITING-LOOP-013`; `AC-WRITING-013.1`–`AC-WRITING-013.10`.
- **Area summary:** A supported dual source container remains ordered member identities plus wrapper provenance, independent of the derived active left/right projection. A currently valid two-cue/member pattern may compose as two columns; invalid types deactivate that projection without losing or unwrapping the source container. Editing and layout compose around ordinary screenplay elements without inventing nested screenplay text.

## Shared composition

- [Document Model](../components/document-model.md) owns canonical group values and semantics in [`dual_dialogue.rs`](../../../crates/scriptcraft-model/src/dual_dialogue.rs), composed with ordinary Body owners in [`project.rs`](../../../crates/scriptcraft-model/src/project.rs). [Editing session](../components/editing-session.md) owns group toggles, ordinary edits, selection, and history over those values; [FDX preservation and storage](../components/fdx-preservation-and-storage.md) separates model-owned retained wrapper/alternate source, FDX codec work, and session-owned publication; [Layout and pagination](../components/layout-and-pagination.md) owns editor and Print geometry.
- [Command contract](../components/command-contract.md) carries the Body-only toggle activation; [Desktop adapters](../components/desktop-adapters.md) owns active-group presentation and pointer/navigation input; [Exports and print](exports-and-print.md) documents the bounded output consumers of Print composition.

## Capability realization

### Group model, editing, and source preservation

- **Requirements:** `REQ-WRITING-LOOP-013`; `AC-WRITING-013.1`–`AC-WRITING-013.3`, `AC-WRITING-013.9`–`AC-WRITING-013.10`.
- **Composition and flow:** The session recognizes/toggles only the bounded exact-capability two-cue pattern over flat Body candidates. The source group records ordered member IDs and wrapper provenance once; active left/right sides derive from those members only when current definitions grant Character capability to exactly two cues and Dialogue or Parenthetical capability to every other member. Active sides are derived values, never persisted membership. A neighboring flat pattern or `PaginateAs` value does not create active source membership; only an explicit eligible toggle records a candidate's ordered members as a group. A type-only mutation or definition remap can deactivate projection while preserving a valid group and all member/wrapper source; it does not invoke structural unwrap. The command boundary dispatches the same parameterless operation from the Format menu, supported shortcut, or other enabled control, with current Body eligibility rechecked at execution. Transactional structural edits retain a valid group or remove affected grouping and perform the ordinary edit as one history operation. The preservation component maps the supported wrapper and its owned data to the flat semantic model and keeps alternate payload opaque on its Dialogue owner.
- **Boundary:** Unsupported or ambiguous wrappers fail closed; inline alternate data is not screenplay text, selection, layout, or export content. Command identity and envelope version remain compatibility-owned by the command component.

### Editor navigation and fixed Print composition

- **Requirements:** `REQ-WRITING-LOOP-013`; `AC-WRITING-013.4`–`AC-WRITING-013.8`.
- **Composition and flow:** Layout derives two-column geometry only for a currently active left/right projection. A group without that projection remains in source order and its members use ordinary sequential body flow exactly once; no wrapper is unwrapped to obtain that fallback. A project-bound transient active-group choice changes only Body editor composition; pointer selection/navigation maps back to the underlying ordinary elements and clears that choice when focus or position leaves. Print composes the current active projection independently of transient editor selection. PDF and text consume that Print structure and do not expose group boundaries or alternate copies.
- **Boundary:** The typed toggle remains one headless semantic command; active presentation is not part of its payload or persisted project. Exact geometry and safe-source contracts are owned by the linked components rather than duplicated here.

### Phase 4 capability boundary

The session/model derives dual activity from ordered source membership and exact Character, Dialogue, and Parenthetical capabilities. An inactive group remains source-preserved, editable, and sequential; generic Type changes and future definition remaps do not unwrap or delete it, while explicit disable may unwrap it. Final-right Enter uses the current member's exact resolved ReturnKey and stages the new paragraph outside the group. Arbitrary/case-variant keys and `PaginateAs` never create dual semantics. Headless projection and editing behavior are distinct from later desktop focus, physical keyboard, pointer-placement, and visual acceptance.

### Phase 5 geometry boundary

Active dual columns use the same explicit resolved-font environment, per-scalar advances, per-visual-line leading, and effective PageLayout bounds as ordinary layout. Courier/Proportional leading is size-derived from each exact resolved face; DocumentLeading applies +0/+1/−1/−2 points, and each wrapped line takes the maximum of its base style and styles actually present. SpaceBefore is the exact nonnegative whole-unit B/S ratio multiplied by effective base leading; outer group and first visible member spacing combine by maximum, and fresh-page starts suppress top spacing. TopMargin anchors the first line-box top; each baseline uses its retained canonical offset. Dual fit uses the greater downward final natural-bottom extent of the two sides against BottomMargin, while column advance remains the taller leading-derived side height. Line gap, character ink, and occupied-line counts are not fit authorities. Oversized spill is bounded and does not clip glyphs to line boxes. The current General definition and wrapper direct overrides supply only the outer wrapper extent; member definitions/direct overrides supply their own text and paragraph geometry. An inactive source container continues to emit its ordered members exactly once in sequential layout.

## Decisions

- Represent dual dialogue as a bounded grouping over ordinary ordered elements, not a general nested-story editor.
- Keep ordered source membership and wrapper provenance authoritative for the retained container; derive active sides from current types and allow a valid inactive container.
- Keep active-group navigation state transient and isolate it from Print composition.

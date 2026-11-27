# Requirements Area: Story Map

## Area purpose

The Story Map is a resizable Outline Editor strip shared by the Script and Beat Board roots. It presents the screenplay timeline, dynamic outline lanes, placed Beat cards, and a read-only view of canonical script scenes without becoming a screenplay editing surface or replacing the root-view selector.

## Approved target contract

### Visibility and root composition

The Outline Editor is hidden by default. Its visibility is an optional global user preference, `show_story_map`, whose missing-value default is false so existing settings remain compatible. When visible, the strip appears below the toolbar and above the active root viewport in both Script and Beat Board roots. It does not alter screenplay text, page layout, or the root selector.

The strip has a 90-point minimum height. A bottom-edge resize gesture changes only disposable presentation state and clamps to the available root height. The left label column is 90 points wide; the remaining area is the timeline viewport. Dynamic lane rows appear in model order above the Page Ruler. The ruler is 24 points high, the scrollbar row matches the status-panel height, and the dynamic lanes plus Script row evenly divide the remaining height. The Script row is immediately below the ruler. At minimum height these same formulas remain in force and text is clipped rather than changing semantic height.

### Shared Beat ownership and lanes

The existing Beat ListItem/card is the only semantic Beat owner shared by the Beat Board and Story Map. A Story Map placement belongs to that existing card; the Story Map does not copy a title or body, create another Beat entity, own a second selection model, or provide a separate history. Deleting a card removes its placement with the card and retains the board, lanes, and unrelated source.

The Story Map owns an optional durable board identity, target length, and an ordered collection of dynamic lanes. A lane retains its durable UUID, display Label, numeric Level, and exact Type string. Source order determines presentation order. Level is metadata only: it is not an array index, placement key, or substitute for Type. Labels retain their exact text without gaining semantics. Standard fallback lanes are Outline 1 (Level 7, Type/Label `Outline 1`) followed by Outline 2 (Level 5, Type/Label `Outline 2`).

### Target and placement semantics

Target length is a positive integral page count bounded to 10,000 pages; when native target data is absent or unusable, the semantic projection uses a safe 120-page fallback without rewriting retained source. Page 1 begins at zero-based position zero.

Target Script Length can be changed from the ruler action or the Outline Editor context menu; both open the same transient dialog prefilled from the current target. The context menu also exposes the existing Hide Outline Editor visibility action and introduces no setting or visibility semantics. Applying a valid changed count is one ordinary undoable semantic edit and updates the live timeline; applying the current count is a no-op. Invalid input, rejection, cancellation, or operational failure leaves the Project unchanged. A requested target must contain every existing Beat placement, using checked placement ends; an unsafe reduction rejects the complete edit and preserves the prior target and all placements. The UI may report the minimum required whole-page count, but target editing never moves, shortens, unplaces, or deletes Beats to accommodate a reduction. The dialog changes semantic Project, history, and dirty state only. Ordinary Save/Save As remains the sole native FDX publication path, retaining existing detached-output, verification, failure, and source-preservation guarantees.

Each placed Beat occupies one lane and a half-open interval expressed as exact integer eighth-page units: destination lane UUID, start eighth, and positive length eighths. The card has at most one placement. The destination UUID must resolve to exactly one lane, and checked start-plus-length must not exceed the target in eighths. No fractional semantic values, negative values, overflow, validation-time rounding, or implicit quantization are accepted. Presentation may snap a user gesture to eighths, but the stored value is already exact.

The display domain may extend beyond the target for screenplay overflow. Let T be target pages and let D be the greater of T and the ceiling of the canonical final Body line bottom. The timeline retains 240 points per page at maximum zoom and extends its domain width to 240D; a target-only document has maximum width 240T. Placement validity remains bounded by T even where the displayed domain extends beyond it.

### Timeline, zoom, scroll, and ruler

Let W be the positive timeline viewport width after the label column, Z the zoom percentage, and S the horizontal scroll origin in scaled timeline pixels. Unscaled domain width is `B = 2000 × D / T`; fit zoom is `Zfit = 100W / B`; effective minimum zoom is the lesser of Zfit and `12T`; maximum zoom is `12T` percent. Page pitch is `(2000 / T) × Z / 100`, scaled domain width is `B × Z / 100`, and maximum origin is `max(0, scaled domain width − W)`. Screen x is timeline-left plus zero-based page position times page pitch minus S; pointer page position is `(pointer x − timeline-left + S) / page pitch`. Width, zoom, and origin changes clamp immediately. Nonpositive or nonfinite viewport widths are noninteractive and do not replace the last valid persisted width or geometry.

Horizontal wheel/trackpad navigation changes only S and clamps after each change. Zoom is immediate and preserves the page at the viewport center. The scrollbar spans the timeline to the zoom control, uses the visible fraction and a 24-point minimum thumb, and maps its position over the complete origin range. It is visible after qualifying pointer movement or effective origin/zoom change, during capture, and while pointer contact keeps a visible bar active; it auto-hides after one second and hidden thumbs do not intercept input.

The Page Ruler paints one-device-pixel ticks at every eighth-page boundary. Whole-page ticks are 6 points high; other eighth ticks are 3 points high. Page labels use the initial density thresholds: every page at 64 points per page or more; even pages from 32 through less than 64; 1, 5, 10, 15 and every fifth page from 16 through less than 32; and 1, 10, 20, 30 and every tenth page below 16. Labels and ticks at or beyond target use the overflow color role. These density values are visual constants, not native-format compatibility claims. A 250 ms quiet delay governs canonical scene/caret projection refresh after actual text or selection activity.

### Canonical scenes and read-only preview

Script scene intervals and the caret marker derive only from canonical Body pagination and exact Scene Heading semantics. Each scene starts at its heading's first canonical line; it ends at the next scene start, or at the final Body line bottom for the last scene. The caret uses the canonical Body caret/line position. Missing canonical locations are omitted rather than fabricated. No second paginator or screenplay-layout mode is introduced.

The last valid projection remains coherent while the user is active. Body edits, selection/caret changes, undo/redo, and document replacement invalidate the projection; actual keyboard, text, paste, IME, or pointer activity resets the 250 ms quiet deadline. Once due, one canonical layout request replaces the projection atomically. Zoom, scroll, height, Beat placement, and Beat Board gestures do not trigger pagination or scene rebuilds. A failed refresh retains the last valid same-document projection; another document never reuses it.

Scene blocks use the immediate timeline transform and clip to their Script row. Their on-strip heading paint removes whole trailing words until it fits, without an ellipsis or partial word; the canonical heading remains complete. Clicking a block opens one transient read-only preview below it, centered on the block and clamped to root content. The preview body is at most 480 × 200 points, includes the full heading (wrapping only when it cannot fit on one line) and the immediate following Body paragraph when present, and has no caret, selection, IME, clipboard, editing command, or native editor. Outside primary press, Escape, document activation, or loss of its source scene closes it; selecting another scene transfers it.

### Placement interaction and collision behavior

Placed Beat blocks share the ruler and scene timeline transform. A drag preview is transient, and a placement-changing release emits at most one semantic operation. Start and length resolve to nearest eighths with nonnegative half-up ties; starts remain nonnegative and the primary interval remains within target. Tail resize changes length while preserving start. Escape, focus/pointer loss, document/token changes, or incompatible save rebase cancels a preview without a semantic edit.

An unplaced Beat dragged from the Beat Board begins with a one-page (eight-eighth) length, clamped to target. For a move, lower-lane followers whose original starts overlap the primary's original half-open interval move by the same delta. Following considers every lower lane in top-down order; each card moves at most once per planner pass. A move or resize resolves same-lane collisions deterministically to the right, preserving the primary at its requested position and processing moved/displaced upper beats' qualifying lower-lane followers. A resize alone does not initially pull followers; tail growth uses the same same-lane displacement cascade, while shrinking changes only the primary. A lower-lane move never pulls an upper-lane card. If a complete result cannot fit before target, the whole operation is rejected without changing any placement; no primary substitution or out-of-target placement is permitted. One successful result is one ordinary history/dirty update; no-op and rejection leave semantics and session state unchanged.

### Native source, state separation, and failure

Only supported direct native structures become semantics. The winner is the first direct root-namespace `DisplayBoards` container and its first direct same-namespace `DisplayBoard` with exactly one unqualified `Type` equal to `StoryMap`; later Story Map boards remain inert and preserved through Save. The target is a unique direct root-namespace `TargetScriptLength`. Lanes come from the first direct same-namespace `Lanes` child and its direct same-namespace `Lane` children in source order. A placement is recognized only from a semantic Beat ListItem's direct same-namespace `SubordinateTo` child with exact unqualified `Type=PageGoal`, `Order=0`, a uniquely resolvable unqualified `Level` equal to a lane's exact Type, and finite decimal unqualified `Number` and `Length` values exactly representable in eighths and within target. Unsupported, malformed, ambiguous, foreign-namespace, unrelated, later-board, or otherwise unprojected source remains retained and is not normalized by Open.

Board identity, target, ordered lanes, and per-card placements are durable Project semantics and participate in equality, dirty comparison, and ordinary history. Height, Width, horizontal ScrollOrigin, ZoomLevel, visibility, hover, selection, gesture, popup, and projection/cache state are disposable; they are excluded from Project equality, semantic history, and dirty state. Navigation and resizing never dirty the Project or invalidate Body pagination by themselves.

Candidate replacement and output are all-or-nothing. A failed semantic candidate leaves Project unchanged. A failed activation, output, verification, or save leaves the destination and prior live semantic/view/session state unchanged. Detached output preserves unsupported source, unrelated nodes, duplicate later Story Map boards, and foreign graph data; view-only attribute updates do not authorize graph changes. Successful Save/Save As is the only source projection boundary. User-owned review is authoritative for rendered behavior, native persistence/interoperability, and product acceptance.

## Related documents

- [Story Map feature blueprint](../../blueprints/features/story-map.md)
- [Beat Board requirements](./beat-board.md)

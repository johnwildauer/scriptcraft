# V4-WO-001 — Native empty managed paragraphs and text-node round-trip safety

- **Status:** Implementation authored; user-owned validation outstanding (execution recorded in [`fdx-fixes-v4.md`](../fdx-fixes-v4.md))
- **Depends on:** Approved [`fdx-fixes-v4.md`](../fdx-fixes-v4.md)
- **Validation:** User-owned; no agent-run checks or independent validators

## Objective

Make empty managed screenplay paragraphs use the Final Draft native shape—Paragraph without Text children—and eliminate the confirmed empty-text-node expected/actual tree asymmetry. Restore ordinary Save, Save As, reopen, and repeated-save behavior for blank and cleared managed paragraphs without weakening strict semantic or preservation checks.

## Evidence and current defect

- Final Draft emits an empty managed Action Paragraph with no Text child in [`reference2.fdx`](../../../.reference/reference2.fdx:125).
- [`make_semantic_element()`](../../../crates/scriptcraft-core/src/fdx.rs:2147) already accepts a managed Paragraph with no Text children, but it creates semantic runs for every imported Text element even when the complete paragraph is empty.
- [`new_paragraph()`](../../../crates/scriptcraft-core/src/fdx.rs:1213) manufactures an empty run and an empty Text element for a new blank paragraph.
- [`replace_text_element_content()`](../../../crates/scriptcraft-core/src/preservation.rs:1575), [`PreservationState::replace_managed_text_elements()`](../../../crates/scriptcraft-core/src/preservation.rs:677), and [`reconcile_managed_paragraph()`](../../../crates/scriptcraft-core/src/fdx.rs:961) can retain empty text child nodes.
- [`write_node()`](../../../crates/scriptcraft-core/src/fdx.rs:1295) emits no bytes for an empty text node, while [`normalize_text_nodes()`](../../../crates/scriptcraft-core/src/fdx.rs:588) retains that node in the expected tree. The strict comparison at [`fdx.rs`](../../../crates/scriptcraft-core/src/fdx.rs:467) can therefore reject valid emitted XML.

## Scope

1. Apply the no-Text-child empty rule to managed screenplay body Paragraphs and direct managed title-role Paragraphs.
2. Cover new-document generation, imported empty paragraphs, ordinary clear/retype editing, writer reconciliation, and repeated serialization.
3. Canonicalize serializer-invisible empty text nodes for strict source-tree comparison.
4. Keep semantic runs and managed source Text runs synchronized in every supported empty/nonempty transition.
5. Add focused core and filesystem Save/Save As regressions, including the updated Final Draft reference.
6. Amend the durable empty-content and serialization contracts.

## Non-goals

- No SceneProperties ownership change; V4-WO-002 owns that work.
- No global removal of empty Text elements from opaque title-page content, board/list data, nested arc Paragraphs, headers/footers, or unknown structures.
- No change to nonempty run formatting, canonical whitespace, CDATA, comments, processing instructions, mixed content, or opaque-node ordering.
- No relaxation of identity, managed-role, semantic, opaque-tree, destination-conflict, or storage checks.
- No broad Final Draft compatibility claim.

## Implementation requirements

### 1. Establish one canonical empty managed-paragraph projection

When a managed paragraph's canonical text is empty:

- its semantic run projection shall be empty;
- its managed source projection shall contain no direct managed Text children;
- its Paragraph identity, role, source Type, direct paragraph attributes, SceneProperties attachments, and non-Text source-owned children shall remain intact;
- whitespace-only canonical text is not empty and must retain its Text content.

When canonical text is nonempty, existing ordered run/text behavior remains unchanged.

Use a bounded helper or clearly shared reconciliation path rather than independent ad hoc special cases in unrelated edit operations. If semantic text is empty but semantic runs still contain content or the source mapping contains non-removable mixed content, fail transactionally rather than silently discarding data.

### 2. Normalize imported empty managed paragraphs

Update [`make_semantic_element()`](../../../crates/scriptcraft-core/src/fdx.rs:2147) so an imported managed paragraph whose aggregated canonical text is empty has no semantic runs, including when the source contains empty Text elements with formatting attributes. The preservation tree shall continue to retain the original source until managed writer reconciliation; this reader change does not itself delete source nodes.

Do not apply this semantic rule to opaque Paragraphs. Only managed screenplay elements call this projection path.

### 3. Stop generating empty Text elements

Change [`new_paragraph()`](../../../crates/scriptcraft-core/src/fdx.rs:1213) to emit no Text elements when canonical semantic text is empty. Do not manufacture an empty run solely to create a Text element. Nonempty behavior, run attributes, ID allocation, and role mapping remain unchanged.

Update managed writer reconciliation in [`reconcile_managed_paragraph()`](../../../crates/scriptcraft-core/src/fdx.rs:961) so a valid empty semantic paragraph removes direct managed Text children from the candidate while retaining non-Text attachments and structural children. Require the already-synchronized empty semantic/source mapping; do not use this rule to erase nonempty or opaque mixed content.

### 4. Keep edit-time semantic and source state synchronized

Review the shared managed text replacement paths, especially [`replace_text_element_children()`](../../../crates/scriptcraft-core/src/preservation.rs:1459) and the editing candidate that owns the corresponding [`ScreenplayElement`](../../../crates/scriptcraft-core/src/lib.rs:292). When an edit makes the complete managed paragraph empty:

- clear its semantic run projection;
- remove its direct managed Text children through the bounded managed mapping;
- retain the Paragraph and permitted non-Text children;
- leave selection, undo/redo, dirty state, and history transactional.

When text is subsequently inserted, create the required Text element/run using the existing namespace and source-mapping rules. Typing into an originally no-Text Paragraph must remain supported.

Apply the same synchronization to derived plain Cast List replacement in [`replace_cast_list_text()`](../../../crates/scriptcraft-core/src/cast_list.rs:142) if it can produce an empty managed paragraph. Formatted or ambiguous mappings retain their current nonblocking/deferral policy rather than being deleted.

### 5. Canonicalize empty text nodes at the comparison boundary

Update [`normalize_text_nodes()`](../../../crates/scriptcraft-core/src/fdx.rs:588) so serializer-invisible empty text child nodes are omitted before adjacent nonempty text nodes are merged. Apply the same normalization to expected and actual trees.

Do not remove empty CDATA nodes in this comparison helper; CDATA has explicit serialized syntax and different source-preservation semantics. Do not remove whitespace-only text nodes. The enclosing managed or opaque element remains present.

Also prevent straightforward creation of new empty text nodes where practical: when [`replace_text_element_content()`](../../../crates/scriptcraft-core/src/preservation.rs:1575) handles a childless Text element and an empty replacement, leave it childless rather than pushing an empty text node. This prevention supplements, rather than replaces, comparison normalization for existing candidates and cleared later children.

## Regression coverage

Author [`v4_wo001_empty_managed_paragraphs.rs`](../../../crates/scriptcraft-core/tests/v4_wo001_empty_managed_paragraphs.rs) with focused coverage for:

1. A minimal imported blank managed Paragraph with no Text child survives direct serialization, reopen, and a second serialization without gaining Text.
2. An imported blank managed Paragraph with an empty Text child is projected as semantically empty and serializes to the native no-Text shape.
3. A new blank managed body paragraph and direct managed title-role paragraph serialize without Text children.
4. Typing into a no-Text imported Paragraph creates the correct source mapping and saves/reopens with the entered text.
5. Clearing that paragraph removes the managed Text children and semantic runs, preserves Paragraph ID/role/direct attributes, and succeeds through Save, reopen, Save As, and another Save.
6. Undo/redo restores exact empty/nonempty semantic and preservation snapshots.
7. Clearing multi-run text keeps model/source projections synchronized and does not produce a re-read mismatch.
8. Editing adjacent parser-produced text nodes and text plus CDATA preserves canonical content and passes strict re-read.
9. Multi-run plain Cast List correction followed by Save/reopen/Save does not fail on emptied later runs.
10. Whitespace-only canonical text retains Text content; empty CDATA, comments, processing instructions, unknown children, and opaque empty Text controls retain their specified source treatment.
11. Actual managed-source mismatches and opaque mixed-content loss remain fatal.
12. Filesystem Save and Save As publish only complete candidates and leave destination/session unchanged under an injected failure.

Add a reference integration case using [`reference2.fdx`](../../../.reference/reference2.fdx:1) that locates the blank Action by its established source ID, verifies it has no Text child after serialization, and completes reopen plus second serialization. Keep large unrelated reference payload assertions limited to existing ownership requirements; this order is not a full reference compatibility audit.

Update any prior tests that explicitly expect an empty managed Text element. Do not rewrite tests that require empty Text in opaque contexts unless the new contract intentionally covers them.

## Durable documentation

Amend:

- [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md:154) with the managed empty-paragraph/no-Text rule, empty semantic-run projection, whitespace-only distinction, and serializer-invisible empty-text normalization.
- [`technical-context.md`](../../../.forge/blueprints/technical-context.md:43) with the single managed empty-text projection and strict comparison boundary.
- [`product-overview.md`](../../../.forge/requirements/product-overview.md:20) only if the current wording implies that empty managed Text elements must be retained; keep unrelated opaque preservation language intact.

Record implementation and authored-test evidence in [`fdx-fixes-v4.md`](../fdx-fixes-v4.md) without claiming user validation.

## User-owned validation

Run the applicable formatting, lint, build, and workspace test commands, plus the focused V4-WO-001 test target. Then perform the empty-Paragraph Final Draft/ScriptCraft matrix in [`fdx-fixes-v4.md`](../fdx-fixes-v4.md), using disposable copies:

- unchanged Final Draft blank Paragraph → ScriptCraft Save As → reopen → Save;
- ScriptCraft type → Save/reopen → clear → Save As;
- Final Draft type/clear cycle → ScriptCraft Save;
- new blank ScriptCraft managed paragraphs → Final Draft open/save → ScriptCraft reopen;
- opaque empty-Text control remains unchanged.

Expected: no empty managed Paragraph gains a Text child, all ordinary saves succeed, nonempty and whitespace-only content remain exact, and deliberate source/tree corruption still fails transactionally.

## Handoff

Proceed to V4-WO-002 only after this order's implementation and authored regressions are recorded. User review/validation remains outstanding unless the user explicitly records otherwise. If a save failure remains, preserve the exact minimal fixture and failure stage for V4-WO-003 rather than weakening comparisons.

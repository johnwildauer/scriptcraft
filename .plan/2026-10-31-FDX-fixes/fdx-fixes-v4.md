# FDX Fixes v4 — empty managed paragraphs and derived scene arcs

**Status:** Revised strategy approved by the user with three sequential work orders in [`work-orders-v4`](work-orders-v4/). V4-WO-001 and V4-WO-002 implementation/authored-regression records are below; both remain user-unvalidated. The user confirms their fixes did not solve Save, activating V4-WO-003. Stable diagnostic localization and authored tests are recorded below; the concrete residual fixture/mechanism and all user-owned validation remain outstanding. No builds, tests, formatting, lint, desktop checks, or interoperability checks were run; repository policy leaves all review and validation to the user.

## Current blocker and required outcome

The user reports that Save fails for existing documents, whether the FDX originated in ScriptCraft or was imported. The reported detail is:

> FDX save failed; document unchanged; session valid; no complete FDX serialization available; FDX save candidate could not be serialized or re-read; FDX contains a construct that cannot be projected safely.

The required outcome is ordinary Save and Save As for valid reopened FDX documents, including no-op saves, repeated saves, and saves after ordinary edits. Empty managed paragraphs must use the native shape evidenced by Final Draft, and a retained Character's scene arc text must survive unrelated roster edits. These changes must not weaken semantic preservation, identity checks, source-tree integrity, conflict detection, or atomic filesystem publication.

The previously reported repeated opaque title-page UUID blocker appears closed by user observation. V4 does not reopen that allocator or import work unless a failing regression demonstrates a distinct remaining identity defect.

## Executive findings

1. **The main suspect is not every element with no content.** Final Draft represents an empty managed Paragraph with no Text child at [`reference2.fdx`](../../.reference/reference2.fdx:125). The current reader already accepts that shape as an empty semantic paragraph at [`fdx.rs`](../../crates/scriptcraft-core/src/fdx.rs:2200), and the writer already handles a managed paragraph with no Text children when its semantic text and runs are empty at [`fdx.rs`](../../crates/scriptcraft-core/src/fdx.rs:1009).
2. **ScriptCraft currently generates the wrong empty-paragraph shape.** [`new_paragraph()`](../../crates/scriptcraft-core/src/fdx.rs:1213) manufactures one run when the semantic run list is empty and creates a Text element even when that run is empty. The new reference supports changing managed empty paragraphs to emit no Text children.
3. **A separate empty-text-node writer defect is confirmed by source.** Managed-text paths can retain empty text child nodes in the expected preservation tree. Serialization emits zero bytes for those nodes, so the reader cannot reconstruct them. The strict normalized-tree comparison can then reject ScriptCraft's own output. This is a concrete save-blocking mechanism, although source inspection alone does not prove it explains every unmodified-document failure.
4. **The new arc reference is already within the supported recognizer.** In [`arc copy.fdx`](../../.reference/arc%20copy.fdx:15), JOHN has one nested arc Paragraph containing one Text run with Arc 1; ANOTHER TEST and TIM have empty nested Paragraphs. [`recognized_arc_paragraph_content()`](../../crates/scriptcraft-core/src/scene_metadata.rs:501) accepts exactly this bounded shape, including text/run attributes.
5. **The requested arc association behavior is already the implemented v3 contract.** [`reconcile_heading()`](../../crates/scriptcraft-core/src/scene_metadata.rs:203) matches normalized Character names, clones a retained Character's complete nested arc subtree, reorders derived beats, and changes only the derived beat Name. Unrelated Character additions/deletions therefore retain JOHN's Arc 1; removing JOHN removes that derived arc and its text.
6. **No duplicate semantic scene model is needed.** Arc content is already part of [`Project`](../../crates/scriptcraft-core/src/lib.rs:462) through its preservation state. Every editing history [`Snapshot`](../../crates/scriptcraft-core/src/editing.rs:77) clones that complete Project, and save/refresh paths reconcile it indirectly from screenplay edits. The v4 change should clarify ownership and close round-trip gaps rather than add a second arc authority.
7. **The user's new SceneProperties direction changes the non-arc retention contract.** For supported managed Scene Headings, the SceneProperties container and its non-arc fields are generated data; CharacterArcBeat content is the authored association/content to retain. This supersedes the v3 requirement to preserve existing Length, Page, Title, and unknown attributes in that supported container. Page/Length recalculation and direct arc editing remain out of scope.
8. **The generic codec error still hides multiple writer gates.** If failures remain after the empty-text and scene-container corrections, private stage-specific diagnostics must identify the exact gate before any broader writer change.

## 1. Save path and failure boundary

The desktop Save command reaches [`save_fdx_mode_at()`](../../apps/scriptcraft/src/main.rs:321), which delegates to [`save_session()`](../../crates/scriptcraft-core/src/fdx_storage.rs:344). Both Save and Save As then share the following sequence:

1. [`EditingSession::prepare_cast_list_save_candidate()`](../../crates/scriptcraft-core/src/editing.rs:205) clones the live project and runs derived scene/Cast List correction.
2. [`NativeSaveCandidateCodec::serialize()`](../../crates/scriptcraft-core/src/fdx_storage.rs:360) calls [`PreparedFdx::from_project()`](../../crates/scriptcraft-core/src/fdx_storage.rs:137), which calls [`write_v6()`](../../crates/scriptcraft-core/src/fdx.rs:384).
3. The storage layer re-reads the prepared bytes and compares the semantic save projection in [`same_save_projection()`](../../crates/scriptcraft-core/src/fdx_storage.rs:431).
4. Only after all candidate checks succeed does the code call atomic replacement or new-destination installation at [`fdx_storage.rs`](../../crates/scriptcraft-core/src/fdx_storage.rs:414).

The reported wording is the codec branch of [`TransactionalSaveError::fmt()`](../../crates/scriptcraft-core/src/fdx_storage.rs:297), not a destination conflict or storage failure. The live session and destination remain unchanged because the candidate never becomes publishable.

The desktop also runs [`refresh_cast_lists_on_schedule()`](../../apps/scriptcraft/src/main.rs:242). A document open long enough for scheduled refresh may therefore contain derived SceneProperties changes before Save. Reproduction must distinguish an unmodified document from one changed by editing or scheduled derived refresh.

## 2. Empty managed Paragraphs and empty text representation

### Evidence from Final Draft

[`reference2.fdx`](../../.reference/reference2.fdx:125) contains a managed Action Paragraph with a Type and ID but no Text child. The same reference contains many empty opaque Paragraphs in other contexts, also without Text children. This establishes the desired native shape for an empty managed paragraph: retain the Paragraph and its source attributes, but do not manufacture an empty Text element.

The arc reference provides a matching nested example: ANOTHER TEST's arc Paragraph at [`arc copy.fdx`](../../.reference/arc%20copy.fdx:12) has no Text child, while JOHN's nonblank arc Paragraph at [`arc copy.fdx`](../../.reference/arc%20copy.fdx:15) has one.

### Current behavior that should change

[`new_paragraph()`](../../crates/scriptcraft-core/src/fdx.rs:1213) currently creates a Text element for an empty semantic paragraph because it first manufactures an empty run at [`fdx.rs`](../../crates/scriptcraft-core/src/fdx.rs:1222) and then creates the element at [`fdx.rs`](../../crates/scriptcraft-core/src/fdx.rs:1231). It avoids adding a text child node only inside that empty Text element.

[`replace_text_element_children()`](../../crates/scriptcraft-core/src/preservation.rs:1459) creates a Text element when typing into a paragraph that had no Text child. That is correct for nonblank input. Clearing the paragraph later does not remove the managed Text element; [`replace_text_element_content()`](../../crates/scriptcraft-core/src/preservation.rs:1575) merely makes its content empty.

### Confirmed serializer/re-read asymmetry

Several managed-text paths can leave empty text child nodes rather than only an empty Text element:

- [`replace_text_element_content()`](../../crates/scriptcraft-core/src/preservation.rs:1575) replaces the first text/CDATA child and clears every later child. When no child exists, it pushes a new text node even if the replacement is empty.
- [`PreservationState::replace_managed_text_elements()`](../../crates/scriptcraft-core/src/preservation.rs:677) uses the same clear-later-children strategy for derived Cast List replacement at [`preservation.rs`](../../crates/scriptcraft-core/src/preservation.rs:726).
- [`reconcile_managed_paragraph()`](../../crates/scriptcraft-core/src/fdx.rs:961) repeats that strategy during writer reconciliation at [`fdx.rs`](../../crates/scriptcraft-core/src/fdx.rs:1036).
- Creating a Text child for an initially childless managed paragraph explicitly starts with an empty text node at [`preservation.rs`](../../crates/scriptcraft-core/src/preservation.rs:1483).

[`write_node()`](../../crates/scriptcraft-core/src/fdx.rs:1295) emits an empty text node as zero bytes. The reader therefore reconstructs no corresponding child. The expected tree still contains that node, and [`normalize_text_nodes()`](../../crates/scriptcraft-core/src/fdx.rs:588) merges adjacent text nodes but does not remove serializer-invisible empty nodes. The comparison at [`fdx.rs`](../../crates/scriptcraft-core/src/fdx.rs:467) can consequently return the reported UnsupportedConstruct error.

This mechanism is distinct from the valid no-Text Paragraph shape. Both belong in the same focused correction because clearing and re-editing managed paragraphs crosses between them.

### Selected empty-content contract

For managed screenplay body and managed title-role Paragraphs:

- When canonical paragraph text is empty, emit no managed Text children. Retain the Paragraph, its role, ID, supported direct attributes, SceneProperties attachments, and unrelated source-owned attributes.
- When canonical paragraph text is nonempty, retain the ordered managed Text runs and their formatting exactly as required by the existing semantic/source mapping.
- Whitespace-only text is nonempty canonical content and must retain its Text representation.
- Keep semantic runs and source Text runs synchronized when transitioning to or from the empty state. Removing an empty Text element in preservation without also normalizing the semantic run projection would merely move the failure to semantic re-read comparison.
- Normalize only serializer-invisible empty text nodes for expected/actual comparison, or prevent their creation at all managed mutation points. Do not drop nonempty CDATA, comments, processing instructions, whitespace-only text, or unknown children.
- Do not globally strip Text elements from opaque title-page content, board/list content, nested arc Paragraphs, headers/footers, or unknown structures. The new generation/removal rule applies only to managed screenplay paragraphs whose canonical text is empty; opaque source remains preservation-owned.

This contract intentionally changes the managed empty-text projection. It does not authorize blanket removal of empty elements elsewhere in FDX.

### Required core regression scenarios

- Open the blank Action at [`reference2.fdx`](../../.reference/reference2.fdx:125), Save unchanged, reopen, and Save again without adding a Text child.
- Create a new blank managed paragraph in ScriptCraft, Save As, and verify that the Paragraph has no Text child.
- Type into that blank paragraph, Save/reopen, then clear it and Save again; the final Paragraph again has no Text child and no semantic runs.
- Clear text represented by multiple runs and by adjacent parser-produced text nodes; semantic/source runs remain synchronized and strict re-read succeeds.
- Edit text plus CDATA without losing nonempty CDATA or introducing an empty-node comparison mismatch.
- Correct a multi-run plain Cast List and exercise a subsequent Save/reopen/Save sequence.
- Confirm whitespace-only canonical text retains Text content and that opaque empty Text structures outside managed screenplay paragraphs remain unchanged.

## 3. SceneProperties ownership and CharacterArcBeat association

### What the arc reference demonstrates

The scene in [`arc copy.fdx`](../../.reference/arc%20copy.fdx:9) contains three derived beats:

| Derived beat | Nested content | Required association behavior |
|---|---|---|
| ANOTHER TEST | Empty Paragraph at [`arc copy.fdx`](../../.reference/arc%20copy.fdx:12) | Retain an empty arc while that normalized Character remains in the scene. |
| JOHN | Paragraph with formatted Text Arc 1 at [`arc copy.fdx`](../../.reference/arc%20copy.fdx:15) | Keep Arc 1 and its run/Paragraph attributes with JOHN through unrelated roster edits. |
| TIM | Empty Paragraph at [`arc copy.fdx`](../../.reference/arc%20copy.fdx:20) | Retain an empty arc while TIM remains; remove it when TIM is removed. |

The canonical Character cues occur in JOHN, TIM, ANOTHER TEST order at [`arc copy.fdx`](../../.reference/arc%20copy.fdx:33). Reconciliation may therefore reorder derived beats from the file's ANOTHER TEST, JOHN, TIM order into current scene order. That reorder must not move JOHN's content to another name or rebuild JOHN's arc as blank.

### Current implementation already matches the association requirement

[`derived_character_projection()`](../../crates/scriptcraft-core/src/scene_metadata.rs:145) supplies normalized association keys and uppercase derived names. [`reconcile_heading()`](../../crates/scriptcraft-core/src/scene_metadata.rs:203) then:

- retains a matched beat's complete nested Paragraph subtree at [`scene_metadata.rs`](../../crates/scriptcraft-core/src/scene_metadata.rs:338);
- updates only its derived Name and position;
- creates a fresh blank arc and UUID for a new normalized name;
- removes the complete derived arc for a removed name;
- treats a rename to a different normalized key as removal plus a new blank arc rather than transferring content by position or ID.

[`recognized_beat()`](../../crates/scriptcraft-core/src/scene_metadata.rs:457) accepts the arc reference's one-Name-attribute beat, one nested Paragraph, and ordered text runs. The arc Paragraph remains opaque to screenplay body content, so Arc 1 does not become an editable Action or Dialogue element.

The working-session requirement is also already satisfied structurally. [`Project.preservation`](../../crates/scriptcraft-core/src/lib.rs:467) holds the ordered source tree and identity graph, [`EditingSession::refresh_cast_lists()`](../../crates/scriptcraft-core/src/editing.rs:192) publishes reconciled candidates, and history snapshots include the complete Project. Authored tests such as [`unrelated_character_addition_removal_and_reordering_keep_retained_arc_unchanged()`](../../crates/scriptcraft-core/tests/v3_wo0013_scene_arc_content_retention.rs:175) and [`undo_redo_save_and_save_as_keep_character_arc_association_and_content()`](../../crates/scriptcraft-core/tests/v3_wo0013_scene_arc_content_retention.rs:340) express the desired behavior, but remain unexecuted under repository policy.

### Revised SceneProperties ownership boundary

The user's v4 direction is that SceneProperties is generated structure and only CharacterArcBeat association/content needs preservation. For a supported managed Scene Heading, the target boundary is therefore:

- SceneProperties and SceneArcBeats are app-owned generated containers.
- Length, Page, Title, and non-arc container attributes/children may be regenerated under the bounded native generation policy; they are no longer preservation promises in a supported scene.
- CharacterArcBeat membership remains derived from normalized current Character cues.
- A retained Character's supported nested arc Paragraph, text, runs, formatting, attributes, and current ID remain associated by normalized Character name.
- Page/Length recalculation is not added. Generated defaults are storage placeholders, not a claim of accurate Final Draft scene pagination.
- Direct arc editing, beat-board editing, and a second semantic scene database remain out of scope.

This supersedes the existing requirement at [`screenplay-writing-loop.md`](../../.forge/requirements/features/screenplay-writing-loop.md:196) that existing Length, Page, Title, and unknown attributes remain source-owned in every supported scene. It also changes the current implementation at [`scene_metadata.rs`](../../crates/scriptcraft-core/src/scene_metadata.rs:279), which defers a scene when SceneProperties contains anything other than SceneArcBeats and formatting whitespace.

The boundary must remain scoped. Unsupported arc shapes still cannot be safely interpreted as app-owned content. Duplicate beats, multiple nested Paragraphs, unknown non-text arc children, and unsafe identity/reference cases should continue to defer that scene's complete derived update rather than discarding ambiguous authored data. The new regeneration permission applies only to the recognized managed SceneProperties container, not to unknown FDX sections or opaque arc payloads.

### Required arc regression scenarios using the new reference

- Open [`arc copy.fdx`](../../.reference/arc%20copy.fdx:1), run derived refresh, and verify JOHN retains Arc 1, its Paragraph ID, and its Text attributes even if beat order changes.
- Add a Character before and after JOHN in the scene; JOHN's arc remains byte-for-structure equivalent while new Characters receive blank arcs and fresh IDs.
- Delete TIM and ANOTHER TEST; their arcs disappear and JOHN's Arc 1 remains unchanged.
- Delete JOHN; JOHN's complete arc and Arc 1 disappear. Undo/redo restores the exact prior arc subtree and ID.
- Change only JOHN's cue case or add a recognized delivery extension; the derived Name remains uppercase JOHN and Arc 1 remains associated.
- Rename JOHN to a different normalized identity; the old arc text is removed and the new identity receives a blank arc with a fresh ID.
- Save, reopen, Save As, and save a second time; Arc 1 remains associated with JOHN and does not enter screenplay body text.
- Combine a supported arc scene with an unsupported arc scene; regenerate only the supported container while preserving the deferred scene's complete ambiguous subtree.

## 4. Writer gates if failures remain

For a source-backed project, [`write_v6()`](../../crates/scriptcraft-core/src/fdx.rs:384) can produce the same public error at several distinct points:

| Stage | Source | Meaning |
|---|---|---|
| Original source graph | [`fdx.rs`](../../crates/scriptcraft-core/src/fdx.rs:388) | The preservation format, original reader, non-Paragraph identities, references, or board graph diverge from the expected original baseline. |
| Managed source reconciliation | [`reconcile_project_elements()`](../../crates/scriptcraft-core/src/fdx.rs:835) and [`reconcile_managed_paragraph()`](../../crates/scriptcraft-core/src/fdx.rs:961) | A managed role, source Type, element order, run mapping, or text-child shape is inconsistent. |
| Semantic re-read | [`fdx.rs`](../../crates/scriptcraft-core/src/fdx.rs:444) | The serialized bytes do not reconstruct the same semantic title/body projection. |
| Identity re-read | [`fdx.rs`](../../crates/scriptcraft-core/src/fdx.rs:450) | The reader's registered identity set differs from the candidate's registered identity set. |
| Source-tree re-read | [`fdx.rs`](../../crates/scriptcraft-core/src/fdx.rs:467) | The normalized expected preservation tree differs from the normalized tree reconstructed from the emitted bytes. |

The empty-text investigation identifies a concrete source-tree comparison mechanism. It should be fixed first. If the universal report persists, add private stage-specific assertions or diagnostics and a minimal failing fixture before changing another writer invariant. Desktop diagnostics must remain privacy-safe and need not expose screenplay text, XML payloads, paths, or tree dumps.

## 5. Selected v4 implementation sequence

### [V4-WO-001 — Native empty managed paragraphs and text-node round-trip safety](work-orders-v4/WO-001-empty-managed-paragraphs.md)

Implement the empty managed-paragraph contract from Section 2 across generation, managed editing, semantic/source run synchronization, and expected-tree normalization. Add the blank reference, clear/retype, multi-node, CDATA, Cast List, and repeated-save regressions. This is the first order because it addresses the confirmed writer asymmetry and the newly evidenced native shape without changing scene ownership.

### [V4-WO-002 — Generated SceneProperties containers with retained Character arc content](work-orders-v4/WO-002-generated-scene-properties.md)

Implement the revised SceneProperties ownership boundary from Section 3. Regenerate supported non-arc container data while retaining matched CharacterArcBeat subtrees by normalized name. Use [`arc copy.fdx`](../../.reference/arc%20copy.fdx:1) for explicit JOHN/Arc 1 association coverage and retain per-scene deferral for ambiguous arc payloads. Do not add a duplicate semantic scene model, direct arc editing, or Page/Length recalculation.

### [V4-WO-003 — Residual reopened-save localization](work-orders-v4/WO-003-residual-save-localization.md)

The user confirmed that Save still fails after the first two fixes, meeting the entry gate. WO-003 adds private writer-stage diagnostics and stable integer call-site prefixes without a speculative production fix. Its implementation and authored test record are below; the user's concrete failing fixture and validation remain outstanding. Any residual production fix requires a new evidence-bound work order.

The following remain rejected as fixes:

- skipping candidate serialization/re-read;
- converting every writer mismatch into a successful save;
- globally ignoring identity-set or source-tree differences;
- removing all SceneProperties reconciliation or all opaque scene data;
- regenerating unrelated imported IDs;
- treating cosmetic XML formatting as a save precondition;
- claiming blanket Final Draft compatibility from targeted reference checks.

## 6. User-run Final Draft and ScriptCraft interoperability checks

Use disposable copies only. Keep the checked-in references unchanged. Record the Final Draft version, operation order, whether scheduled refresh had time to run, and the exact failing stage/message if Save fails. XML inspection is supplemental; also open the saved file in each application to check visible screenplay and arc behavior.

### A. Empty managed Paragraph round trip

1. **Final Draft baseline:** Open a disposable copy of [`reference2.fdx`](../../.reference/reference2.fdx:1) in Final Draft. Confirm the final Action paragraph is blank, save a separate copy, and inspect that its blank managed Paragraph still has no Text child.
2. **ScriptCraft no-op Save:** Open that Final Draft-saved copy in ScriptCraft. Without editing, Save As to a new disposable FDX, close/reopen the output, and Save again. Expected: all saves succeed; the blank Paragraph remains blank and has no Text child.
3. **ScriptCraft edit cycle:** In another copy, type text into the blank Action, Save, close/reopen, then clear the text and Save As. Expected: the nonblank intermediate file opens with the entered text in both apps; the final blank Paragraph has no Text child.
4. **Final Draft edit cycle:** Open the ScriptCraft final output in Final Draft, type into the same blank Action, save a new copy, then clear it in Final Draft and save another copy. Open both copies in ScriptCraft and Save As. Expected: no serialization failure and no manufactured empty Text element in the final blank Paragraph.
5. **New ScriptCraft content:** Create a new ScriptCraft document with an empty Scene Heading and an empty Action, Save As, and inspect the output. Expected: empty managed Paragraphs have no Text children; SceneProperties remains attached to the empty Scene Heading.
6. **Opaque control:** Use a file with empty Text elements in opaque title/header/board content. Expected: the managed-paragraph cleanup does not strip or rewrite those unrelated opaque structures.

### B. JOHN Character arc association

1. **Baseline in both apps:** Open a disposable copy of [`arc copy.fdx`](../../.reference/arc%20copy.fdx:1) in Final Draft and ScriptCraft. In Final Draft, use its Character arc presentation to confirm JOHN has Arc 1 and the other two Characters have blank arcs. In ScriptCraft, confirm Arc 1 does not appear as screenplay body content.
2. **Unchanged ScriptCraft Save:** Save As from ScriptCraft, close/reopen, and Save again. Open the output in Final Draft. Expected: JOHN still has Arc 1; ANOTHER TEST and TIM remain blank; Cast List and canonical cues remain correct.
3. **Unrelated addition:** In ScriptCraft, add a new Character before JOHN and another after JOHN, then Save. Expected: the new Characters receive blank arcs; JOHN retains Arc 1; no arc content transfers by position.
4. **Unrelated deletion:** Delete TIM and ANOTHER TEST in separate disposable scenarios, saving and reopening each. Expected: only the removed Character's arc disappears; JOHN retains Arc 1.
5. **Case/extension association:** Change JOHN's cue casing or add a recognized delivery extension such as (V.O.), then Save. Expected: the derived beat remains JOHN and Arc 1 survives; canonical cue text retains the edited form.
6. **Actual rename:** Rename JOHN to a distinct Character name and Save. Expected: the old JOHN arc and Arc 1 are removed; the new name receives a blank arc. Verify this in Final Draft as well as in XML.
7. **History:** Repeat addition/deletion/rename with undo and redo before Save. Expected: the exact prior arc association and content return, including Arc 1 and its nested ID.
8. **Final Draft-authored arc update:** In Final Draft, change JOHN's arc text to a distinct disposable value and save a new copy. In ScriptCraft, add/delete an unrelated Character and Save. Reopen in Final Draft. Expected: the new JOHN arc text survives the unrelated ScriptCraft roster edit.
9. **Two-way repeated save:** Save the result again from ScriptCraft, reopen it in Final Draft, save a separate Final Draft copy, and reopen that copy in ScriptCraft. Expected: association and arc text remain intact without a save refusal.

### C. Generated SceneProperties container

1. Start with a supported scene whose SceneProperties has nondefault Length, Page, Title, and generated non-arc children, plus a JOHN arc containing text.
2. Edit an unrelated Action or add/delete a non-JOHN Character in ScriptCraft and Save.
3. Expected under the revised v4 contract: the supported container follows the generated policy, while JOHN's arc text and association remain intact. No accurate Page/Length recalculation is implied.
4. Open the result in Final Draft, save a separate copy, and reopen in ScriptCraft. Record any Final Draft regeneration of container fields or nested IDs without treating nested ID stability as the association key.
5. For an ambiguous/unsupported arc control, expected behavior remains per-scene deferral: the ambiguous subtree is preserved rather than discarded, and unrelated canonical edits still Save.

### D. Residual failure isolation

If any save still fails, reduce the fixture in this order:

1. Action-only document with no Scene Heading, Cast List, or arc data.
2. Blank Action-only document with no Text child.
3. Scene Heading with empty SceneArcBeats.
4. Scene with JOHN/Arc 1 but no other Characters.
5. Full arc reference.
6. Full real-world reference.

The first failing reduction identifies whether the residual issue is general managed-text serialization, empty content, scene-container reconciliation, arc content, or unrelated opaque FDX data. Do not respond by weakening all strict comparisons.

## Durable documents to amend during implementation

- [`screenplay-writing-loop.md`](../../.forge/requirements/features/screenplay-writing-loop.md:154): specify no Text children for empty managed paragraphs, serializer-invisible empty-text canonicalization, and the revised generated SceneProperties/non-arc ownership boundary.
- [`product-overview.md`](../../.forge/requirements/product-overview.md:31): replace the current SceneProperties retention wording with generated container ownership and normalized Character-associated arc content retention.
- [`technical-context.md`](../../.forge/blueprints/technical-context.md:43): document managed empty-text projection, generated SceneProperties containers, and the existing Project/preservation/history ownership path without introducing a second arc authority.

These amendments are required because the revised v4 direction intentionally supersedes the current supported-scene Length/Page/Title/unknown-attribute preservation contract. They must not weaken preservation for unrelated opaque FDX structures or ambiguous arc payloads.

## Validation status and closeout criteria

No validation has been run for this investigation. Existing and newly proposed tests are authored scenarios, not passing results. After implementation, the user owns all command checks, desktop checks, and Final Draft interoperability checks.

Minimum closeout criteria:

- unchanged reopened Save and Save As succeed;
- ordinary edited Save and Save As succeed;
- a second save after reopening the first output succeeds;
- empty managed Paragraphs have no Text children while whitespace-only and nonempty content remain intact;
- JOHN's Arc 1 survives unrelated Character additions, deletions, reordering, case/extension changes, Save, Save As, reopen, and two-way Final Draft checks;
- Character removal/rename follows the specified derived-arc removal/new-blank behavior;
- supported SceneProperties container regeneration does not alter ambiguous unsupported arc subtrees or unrelated opaque FDX data;
- deliberate source/identity/tree corruption still fails transactionally;
- failed Save and Save As leave both destination and live session unchanged.

Until those results exist, the empty-text defect and current nonconforming generation behavior are confirmed by source/reference inspection, the arc association design is supported by existing code and authored tests, and the precise cause of every user-observed save failure remains subject to the focused validation matrix rather than a blanket claim.

## V4-WO-001 execution log — 2026-11-02

- **Scope executed:** Native no-`Text` projection for empty managed body/title-role Paragraphs; imported empty-run projection; managed writer reconciliation; edit-time semantic/source synchronization for empty/nonempty transitions; safe plain Cast List empty replacement; strict source-tree normalization for serializer-invisible empty XML text nodes. Empty CDATA and opaque/mixed content remain distinct and are not normalized away. No SceneProperties ownership changes were made.
- **Implementation authored:** New blank Paragraph generation emits no run or `Text` child. Imported empty Text runs produce no semantic runs, while preservation remains unchanged until reconciliation. Clear/retype paths remove managed source Text children when canonical text becomes empty, clear semantic runs, and create Text elements on subsequent insertion. Empty-run Cast List correction is bounded to plain, unformatted mappings. Writer reconciliation rejects inconsistent semantic/source state or unsupported inline children instead of silently discarding them.
- **Regression coverage authored, not executed:** [`v4_wo001_empty_managed_paragraphs.rs`](../../crates/scriptcraft-core/tests/v4_wo001_empty_managed_paragraphs.rs) covers the blank Action in `reference2.fdx`, imported styled-empty Text, generated blank body/title role, type/clear/retype, undo/redo snapshots, multi-run clear, strict comparison with injected empty text nodes, opaque child preservation, text/CDATA edits, whitespace-only preservation, and reference reopen/re-serialization. Focused filesystem unit coverage was added for empty Paragraph Save/Save As and injected partial-write failures, plus multi-run plain Cast List correction and repeated Save in `fdx_storage.rs`.
- **Durable contract updates:** `screenplay-writing-loop.md` and `technical-context.md` now define empty managed Paragraph projection and the comparison boundary. Product overview was not changed because its current wording does not require empty managed Text elements to be retained. The WO status records implementation authorship and outstanding user validation.
- **Validation/policy:** No tests, builds, formatting, lint, manual verification, implementation checks, reviews, or independent validators were run, in compliance with repository policy. Authored tests are not evidence of passing behavior; user validation and Final Draft interoperability checks remain outstanding.
- **Scope handoff/blockers:** V4-WO-002 and V4-WO-003 were not implemented. No implementation blocker was established by permitted inspection; behavior remains unverified until user-owned checks run. No residual save failure is claimed closed.

## V4-WO-002 execution log — 2026-11-02

- **Scope executed:** Supported managed SceneProperties and SceneArcBeats are regenerated containers using shared native defaults. Length is `1/8`, Page is `1`, Title is empty, and element namespace follows the managed heading. Non-arc attributes and children no longer cause deferral and are removed from supported containers during refresh. The existing per-scene detached candidate boundary, duplicate/namespace-conflict checks, arc recognizer, Paragraph ID/reference safety, and atomic deferral remain.
- **Arc ownership:** Supported CharacterArcBeat subtrees continue to be matched by normalized Character key. Matched nested Paragraphs, IDs, attributes, ordered Text/CDATA runs, text, and run formatting are cloned intact while derived Name/order is updated. New identities receive blank arcs; removed identities lose their derived arcs; unsupported or ambiguous arc payloads keep the original complete scene subtree. Generated non-arc container policy is shared with new-document generation; no parallel arc model was introduced.
- **Regression coverage authored, not executed:** [`v4_wo002_generated_scene_properties.rs`](../../crates/scriptcraft-core/tests/v4_wo002_generated_scene_properties.rs) uses the checked-in `arc copy.fdx` reference. It covers JOHN/Arc 1 and nested ID retention through canonical beat reordering and repeated serialization, supported nondefault container regeneration with an extra generated child, before/after roster additions, unrelated Character removals, normalized case/extension edits, actual rename to a blank fresh-ID arc, ambiguous scene subtree deferral, and native Save/reopen/Save As/repeated Save. The prior internal container-retention assertion was revised to the generated default contract. Existing v3 tests remain authored evidence for history snapshots, independent scenes, ambiguous payload varieties, and unsafe-reference deferral; none of these tests has been run by the agent.
- **Durable contract updates:** [`screenplay-writing-loop.md`](../../.forge/requirements/features/screenplay-writing-loop.md), [`product-overview.md`](../../.forge/requirements/product-overview.md), and [`technical-context.md`](../../.forge/blueprints/technical-context.md) now define generated supported-scene containers and retained normalized-name arc ownership. The v3 promise to preserve supported-scene Length/Page/Title/unknown container data is explicitly superseded; ambiguous arc data and unrelated opaque FDX data remain preservation-owned.
- **Validation/policy:** No tests, builds, formatting, lint, manual verification, implementation checks, reviews, or independent validators were run, in compliance with repository policy. Authored tests are not passing evidence. User-owned validation must include the reopened-save and Final Draft interoperability matrices in this plan.
- **Blockers and handoff:** No implementation blocker was established by permitted source inspection. Save behavior, generated-container acceptance in Final Draft, namespace variants, history/save integration, and all other behavior remain unverified. V4-WO-003 was not implemented, and no residual save failure is claimed closed.

## V4-WO-003 execution log — 2026-11-02

- **Activation:** The user confirms that V4-WO-001 and V4-WO-002 did not solve the Save failure. The conditional entry gate is met; WO-003 is implemented as diagnostic localization only.
- **Diagnostic behavior:** Save/Save As writer and storage `UnsupportedConstruct` returns/propagations now use stable integer prefixes. The complete 1–34 function/call-site map is centralized in [`technical-context.md`](../../.forge/blueprints/technical-context.md). `TransactionalSaveError` and the desktop error detail preserve the underlying `INTEGER: FDX contains a construct that cannot be projected safely` text. Generic non-save reader/import `UnsupportedConstruct` and other error categories retain their existing messages/categories.
- **Private diagnostics:** The core writer helper records bounded stages; strict normalized source-tree mismatches can provide a crate-private first-difference path, node kind, expanded name, and counts after the mismatch. Identity mismatches provide only domain and missing/unexpected direction. No text, attribute values, XML, paths, or identity values are emitted to users. Strict semantic, identity, source-tree, and transaction gates are unchanged.
- **Authored coverage, not executed:** [`v4_wo003_residual_save_localization.rs`](../../crates/scriptcraft-core/tests/v4_wo003_residual_save_localization.rs) exercises unchanged reopened-reference Save, Save As, and second-Save controls, plus exact code 23 across Save and Save As failures and transaction atomicity/prefix retention. Private unit assertions in [`fdx.rs`](../../crates/scriptcraft-core/src/fdx.rs) cover stable unique code numbers and per-code assignments, exact prefix, a generated-document writer stage, and value-free structural/identity mismatch summaries. A desktop unit assertion covers detail-path prefix retention. Existing writer-rejection expectations were updated with their diagnostic codes.
- **Residual mechanism/blocker:** No concrete user-provided FDX fixture, exact failed operation sequence, original error, or failure phase accompanied the confirmation. Therefore the synthetic code-23 test is contract coverage only, not a reproduction; no actual residual mechanism or production follow-up is asserted. The concrete failure must still be localized by the user-run reopened-save matrix and reported prefix.
- **Validation/policy:** No tests, builds, formatting, lint, manual verification, implementation checks, reviews, or independent validators were run. All authored tests and desktop/Final Draft Save, Save As, reopen, and interoperability validation remain user-owned. WO-003 is not closed and no Save fix is claimed.

# V4-WO-002 — Generated SceneProperties containers with retained Character arc content

- **Status:** Implementation and focused regression authorship recorded; user validation outstanding
- **Depends on:** V4-WO-001 implementation and authored regression record
- **Validation:** User-owned; no agent-run checks or independent validators

## Objective

Treat SceneProperties and SceneArcBeats as app-owned generated containers for supported managed Scene Headings while preserving each retained Character's supported CharacterArcBeat subtree by normalized Character name. Regenerate non-arc container data under the native default policy, retain JOHN's Arc 1 through unrelated roster edits, and keep ambiguous arc payloads safely deferred.

## Evidence and current behavior

- [`arc copy.fdx`](../../../.reference/arc%20copy.fdx:15) stores JOHN's Arc 1 in one nested Paragraph with one formatted Text run. ANOTHER TEST and TIM have empty nested arc Paragraphs.
- [`recognized_beat()`](../../../crates/scriptcraft-core/src/scene_metadata.rs:457) and [`recognized_arc_paragraph_content()`](../../../crates/scriptcraft-core/src/scene_metadata.rs:501) already support this shape.
- [`reconcile_heading()`](../../../crates/scriptcraft-core/src/scene_metadata.rs:203) already retains matched nested arc subtrees by normalized name, adds new blank arcs, removes stale derived arcs, and reorders beats from current Character cues.
- The same function currently preserves existing SceneProperties attributes and rejects any non-arc child other than formatting whitespace at [`scene_metadata.rs`](../../../crates/scriptcraft-core/src/scene_metadata.rs:279).
- The current durable requirement at [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md:196) explicitly preserves Length, Page, Title, and unknown attributes. The approved v4 direction supersedes that contract for supported managed scenes.
- Arc content already belongs to the working document through [`Project.preservation`](../../../crates/scriptcraft-core/src/lib.rs:467) and history [`Snapshot`](../../../crates/scriptcraft-core/src/editing.rs:77). No second semantic arc database is needed.

## Scope

1. Separate generated SceneProperties/SceneArcBeats container reconciliation from supported CharacterArcBeat content retention.
2. Regenerate supported non-arc container attributes and children using the existing native defaults.
3. Retain matched arc Paragraph IDs, attributes, ordered runs, text, CDATA, and run formatting by normalized Character association.
4. Preserve per-scene deferral for ambiguous or unsupported arc payloads and unsafe identity/reference cases.
5. Exercise scheduled refresh, undo/redo, Save, Save As, reopen, and repeated save using the new arc reference.
6. Amend all durable SceneProperties ownership language.

## Non-goals

- No direct Character arc editor or beat-board editing UI.
- No SceneProperties Page/Length recalculation or claim of accurate Final Draft scene pagination.
- No duplicate semantic scene/arc model outside the existing Project preservation/history ownership path.
- No association by nested UUID, beat position, or full delivery-extension-bearing cue text.
- No deletion of ambiguous arc payloads merely because supported containers are now generated.
- No change to unrelated opaque FDX sections, board/list data, title-page content, or ordinary screenplay UUID behavior.
- No V4-WO-001 empty managed-paragraph implementation beyond preserving its results.

## Implementation requirements

### 1. Split container support from arc support

Refactor [`reconcile_heading()`](../../../crates/scriptcraft-core/src/scene_metadata.rs:203) into clear bounded stages:

1. Locate the managed heading and its same-namespace SceneProperties/SceneArcBeats structure.
2. Validate that the structure is unambiguous enough to identify the derived arc container: at most one relevant SceneProperties and at most one relevant SceneArcBeats. Multiple or namespace-conflicting containers remain unsupported and defer the scene.
3. Parse and validate every CharacterArcBeat under the supported arc rules before publishing any candidate change.
4. Reconcile arc membership/content in an isolated scene candidate.
5. Rebuild the supported non-arc container from the generated native policy and install the reconciled arc container at the existing managed heading position, before the first managed Text child when creation is required.

Non-arc SceneProperties attributes or children must no longer, by themselves, cause deferral in an otherwise supported scene. Their regeneration is an intentional v4 derived-data change, not silent loss of unknown FDX content elsewhere.

### 2. Regenerate only supported non-arc container data

Use the existing native default construction in [`scene_properties_for()`](../../../crates/scriptcraft-core/src/scene_metadata.rs:382) and [`new_scene_properties()`](../../../crates/scriptcraft-core/src/scene_metadata.rs:184) as the shared policy source:

- Length defaults to 1/8;
- Page defaults to 1;
- Title defaults to empty;
- SceneArcBeats remains the derived arc container;
- element names and namespace follow the managed heading.

Avoid divergent default tables between new-document generation and imported-scene reconciliation. Refactor shared helpers if necessary so both paths emit the same generated container shape.

Do not recalculate Page or Length from ScriptCraft pagination. Do not retain old nondefault values merely for round-trip stability; the approved v4 contract makes them generated placeholders in supported scenes.

### 3. Preserve supported arc content by normalized Character name

Retain the existing normalized projection from [`derived_character_projection()`](../../../crates/scriptcraft-core/src/scene_metadata.rs:145) and its uppercase stored derived names. For each scene:

- match existing beats by normalized Character key, not nested UUID or position;
- clone a matched beat's complete nested Paragraph subtree and update only derived beat Name/order;
- retain Arc 1 and all supported run/Paragraph attributes for retained JOHN;
- add new normalized names with blank arcs and bounded fresh UUID v4 nested Paragraph IDs;
- remove unmatched derived arcs and their nested text only after existing unique-ID/reference-safety checks;
- treat a rename to a different normalized key as removal plus a new blank arc;
- keep arc Paragraphs opaque to screenplay body content.

The arc reference's source beat order differs from canonical Character order. Reordering must therefore be covered explicitly and must not rebuild JOHN's retained arc as blank.

### 4. Keep unsupported arc scenes atomic and deferred

Continue returning [`SceneMetadataError::UnsupportedSceneMetadata`](../../../crates/scriptcraft-core/src/scene_metadata.rs:21) for ambiguous arc payloads, including duplicate normalized beats, multiple nested Paragraphs, unknown non-text arc children, missing/ambiguous required IDs, and unsafe external references.

On deferral:

- discard the complete attempted scene candidate;
- retain the original SceneProperties/SceneArcBeats subtree, including its non-arc fields and ambiguous payload;
- leave that scene's Cast Lists unchanged;
- continue independent supported scenes and canonical native Save/Save As.

This deferral boundary prevents the new generated-container permission from becoming permission to discard unrecognized authored arc data.

### 5. Preserve existing session and save ownership

Continue using [`reconcile_project_with_outcome()`](../../../crates/scriptcraft-core/src/scene_metadata.rs:39) as the detached per-scene candidate boundary. Scheduled refresh, undo/redo correction, and [`prepare_save_candidate()`](../../../crates/scriptcraft-core/src/cast_list.rs:101) must consume the same result.

Do not add a parallel arc cache or derive arc text from semantic screenplay elements. The preserved nested arc subtree remains the retained content authority; normalized Character names are the association keys.

Keep strict model/preservation validation, identity re-read, source-tree re-read, conflict detection, and atomic publication unchanged.

## Regression coverage

Author [`v4_wo002_generated_scene_properties.rs`](../../../crates/scriptcraft-core/tests/v4_wo002_generated_scene_properties.rs) with focused fixtures plus the real arc reference:

1. A supported scene with nondefault Length/Page/Title and additional generated non-arc children regenerates to the shared defaults while its supported arcs reconcile successfully.
2. New-document and imported-scene generation emit the same SceneProperties default structure and namespace behavior.
3. [`arc copy.fdx`](../../../.reference/arc%20copy.fdx:1) imports with three opaque arc Paragraphs and no Arc 1 screenplay body element.
4. Derived refresh may reorder ANOTHER TEST/JOHN/TIM into canonical scene order, but JOHN retains Arc 1, nested Paragraph ID, Paragraph attributes, Text attributes, and exact text.
5. Adding Characters before and after JOHN creates fresh blank arcs without changing JOHN's retained subtree.
6. Removing TIM and ANOTHER TEST removes only their derived arcs and leaves JOHN/Arc 1 intact.
7. Removing JOHN removes the complete JOHN arc and Arc 1; undo/redo restores its exact prior subtree and ID.
8. Case-only and recognized delivery-extension cue changes retain the JOHN association and Arc 1 while canonical cue text remains exactly edited.
9. Renaming JOHN to a different normalized identity removes Arc 1 and creates a blank arc with a fresh ID.
10. Save, reopen, Save As, and a second Save preserve JOHN/Arc 1 and the regenerated supported container.
11. The same Character name in independent scenes retains independent arc content.
12. Duplicate beats, multiple nested Paragraphs, unknown non-text arc children, and unsafe references defer only the affected scene and preserve its complete original subtree.
13. A supported scene beside a deferred scene regenerates only the supported container while unrelated canonical edits still Save and Save As.
14. Existing fatal managed-source mismatch, required-ID allocation, identity/tree re-read, destination conflict, and storage failure behavior remains intact.

Reuse and update V3 arc tests where their expectations remain correct. Replace only assertions that require supported-scene non-arc Length/Page/Title/unknown-attribute preservation; retain all ambiguous-payload preservation assertions.

## Durable documentation

Amend:

- [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md:196) to define generated supported SceneProperties/SceneArcBeats containers, default non-arc fields, normalized name-associated arc content retention, and unchanged ambiguous-scene deferral.
- [`product-overview.md`](../../../.forge/requirements/product-overview.md:31) to remove the supported-scene Length/Page/Title/unknown-attribute preservation promise while retaining the arc-content and canonical-text contracts.
- [`technical-context.md`](../../../.forge/blueprints/technical-context.md:43) to describe container regeneration, supported arc ownership, and the existing Project/preservation/history path without a second arc authority.

Record implementation and authored-test evidence in [`fdx-fixes-v4.md`](../fdx-fixes-v4.md), explicitly noting the superseded v3 non-arc retention rule and the absence of agent-run validation.

## User-owned validation

Run the applicable formatting, lint, build, workspace, and focused V4-WO-002 tests. Then execute the JOHN Character arc and generated SceneProperties matrices in [`fdx-fixes-v4.md`](../fdx-fixes-v4.md) with disposable files and both applications:

- unchanged arc reference → ScriptCraft Save As → reopen → Save → Final Draft open;
- unrelated Character additions/deletions before and after JOHN;
- JOHN case/extension change versus actual rename;
- JOHN deletion plus undo/redo;
- Final Draft-authored JOHN arc update → unrelated ScriptCraft roster edit → Final Draft reopen;
- supported nondefault container regeneration without Page/Length recalculation claims;
- ambiguous arc control remains deferred and preserved.

Expected: JOHN's current arc text remains associated with JOHN through every retained-Character scenario, unsupported arc data is not guessed away, and ordinary native saves complete without weakening strict candidate checks.

## Handoff

After implementation and authored regression recording, run the user-owned reopened-save matrix. If any existing-document save failure remains, preserve the smallest failing fixture and exact codec stage for V4-WO-003. If no failure remains, V4-WO-003 records closure rather than introducing speculative changes.

## Execution record — 2026-11-02

- Implemented shared generated defaults for new and imported scene containers. Supported imported SceneProperties non-arc attributes and children are regenerated; the existing per-scene candidate boundary and ambiguity checks remain in place.
- Added focused regression authorship using `.reference/arc copy.fdx` for the three derived beats, JOHN's Arc 1 subtree and ID, source-order reordering, generated defaults, roster additions/removals, normalized cue changes, rename behavior, ambiguous arc deferral, Save/reopen/Save As/repeated Save, and generated non-arc field removal. Updated the prior internal regression that required supported-scene container-field retention.
- Updated the durable writing-loop requirement, product overview, technical context, and v4 execution log to supersede the v3 non-arc retention rule.
- No tests, builds, formatting, lint, manual verification, implementation checks, reviews, or independent validators were run, per repository policy. Authored regressions remain unexecuted and user-owned validation is outstanding.
- No implementation blocker was established by permitted source inspection. Save and Final Draft interoperability behavior remain unverified. V4-WO-003 was not implemented.

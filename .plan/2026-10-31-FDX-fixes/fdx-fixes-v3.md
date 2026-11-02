# FDX Fixes v3 — native round-trip blockers and derived-data ownership

**Status:** Source-inspected strategy record revised with user-observed Final Draft behavior and a concrete failing reference. No implementation work orders are approved here. No builds, tests, desktop checks, or additional Final Draft interoperability checks were run; repository policy leaves review and validation to the user.

## Current priorities and resolved items

1. Restore ordinary Save and Save As for existing FDX documents without weakening canonical screenplay preservation or transactional filesystem safety.
2. Open the real-world Final Draft reference without rejecting legal repeated IDs in opaque title-page content.
3. Treat supported SceneProperties scene-cast data and Cast Lists as app-owned derived projections after import, while retaining each retained Character's arc content.
4. Stop cosmetic XML formatting from blocking an otherwise valid native save.
5. Keep the distinction between ScriptCraft's managed screenplay projection and the larger preserved FDX document explicit.

User-confirmed closures and boundaries:

- UUID allocation for screenplay elements is working: established screenplay IDs remain unchanged, new elements receive new IDs, and practical collision risk is acceptable. No change to that allocator or ordinary screenplay-ID lifecycle is needed in v3.
- Status-bar legibility is acceptable for now. No status-bar redesign is needed in v3.
- User observation shows that Final Draft regenerates nested scene-arc Paragraph IDs even on an otherwise unchanged save. Those derived nested IDs therefore must not be used as durable character-association or ownership evidence. This is a scoped distinction, not a request to regenerate ordinary screenplay Paragraph IDs.
- PDF non-ASCII substitution, attached-scene structural split/join limitations, and broad Final Draft application compatibility remain separate recorded limitations.

## Executive findings

- **The scene-arc ownership rule is a real save blocker.** Every Save and Save As runs derived scene/cast reconciliation before serialization. Under the current runtime-only creation-ownership rule, a stale arc after reopen returns an error and prevents the entire save.
- **The ownership premise is too conservative for supported derived scene data.** Final Draft behavior and the user's product direction establish that supported SceneProperties scene-cast data becomes app-owned after deserialization. It may be added, changed, and removed in response to Character edits; no ScriptCraft creation marker is needed.
- **Association must use a normalized Character name, not the nested UUID or full cue text.** The real script stores canonical cue HELEN MIRREN (V.O.) but derived arc Name HELEN MIRREN. A retained Character's nested arc content must survive unrelated Character additions and deletions. A removed Character's derived arc and arc text are removed with it.
- **Derived names use stored uppercase, while canonical screenplay text retains its typed case.** CharacterArcBeat Name values and generated Cast List text should be uppercase derived data. This does not reactivate edit-time uppercasing of Character elements or any other canonical screenplay text.
- **The real-script import failure has a concrete repeated-ID cause.** Its title page repeats one Paragraph ID across many opaque paragraphs. The reader currently requires every Paragraph identity to be globally unique and returns AmbiguousIdentity.
- **Imported Content formatting is an independent structural-edit save gate.** Newly inserted or split paragraphs must currently receive cosmetic sibling whitespace. Ambiguous mixed Content or effective XML space preservation can therefore make serialization fail despite a valid managed source representation.
- **FDX and ScriptCraft should not have identical structural coverage.** FDX is broader than the editor. The invariant is precise managed edits plus safe preservation or explicit derived regeneration, not forcing every imported document into ScriptCraft's generated subset.

## 1. Derived SceneProperties, scene arcs, and Cast Lists

### Current save-blocking call path

[`save_session_with_codec()`](../../crates/scriptcraft-core/src/fdx_storage.rs:375) calls [`EditingSession::prepare_cast_list_save_candidate()`](../../crates/scriptcraft-core/src/editing.rs:205) before serializing either Save or Save As. That reaches [`correct_cast_lists()`](../../crates/scriptcraft-core/src/cast_list.rs:37), which first calls [`reconcile_project()`](../../crates/scriptcraft-core/src/scene_metadata.rs:29).

The current implementation then creates document-wide failure gates:

| Gate | Current source behavior | Current consequence |
|---|---|---|
| Stale arc after reopen | [`reconcile_heading()`](../../crates/scriptcraft-core/src/scene_metadata.rs:351) requires runtime creation proof before removing an unmatched arc. Import initializes that proof empty in [`read_v6()`](../../crates/scriptcraft-core/src/fdx.rs:362). | A normal Character rename/delete after reopen can block every later Save and Save As. |
| Unsupported arc payload | [`recognized_beat()`](../../crates/scriptcraft-core/src/scene_metadata.rs:482) accepts only an empty nested Paragraph with one exact attribute shape. Text-bearing arc content and other imported variations can be rejected. | Derived arc content cannot participate in normal reconciliation and can block saving. |
| Formatted stale Cast List | [`replace_cast_list_text()`](../../crates/scriptcraft-core/src/cast_list.rs:129) rejects run attributes rather than replacing derived text. | A needed Cast List correction can block saving unrelated edits. |

The old expectation is explicit in [`separately_reopened_stale_imported_scene_arc_safe_stops_without_mutation()`](../../crates/scriptcraft-core/tests/wo0032_uuid_lifecycle.rs:293). That expectation is now superseded for supported derived arcs; it should not be preserved as desired behavior.

### Selected ownership model

Once a supported SceneProperties/SceneArcBeats structure is deserialized, its scene-cast projection is **derived data owned by the current application session and document**, regardless of whether Final Draft or ScriptCraft originally wrote it.

- Remove runtime creation proof as a deletion prerequisite for supported CharacterArcBeat entries. The generated-arc ownership set in [`PreservationState`](../../crates/scriptcraft-core/src/preservation.rs:284) should become unnecessary rather than gaining a durable FDX counterpart.
- Reconcile supported arcs from the current scene's Character elements on refresh, history correction, Save, and Save As.
- Add derived entries for new Characters, retain entries for retained Characters, reorder entries to the supported projection, and remove entries for Characters no longer present.
- Removing a derived Character entry removes its nested arc text as well. This is intentional derived-data maintenance, not deletion of canonical screenplay content.
- Do not infer ownership from nested Paragraph UUID shape. Final Draft's regeneration of these IDs demonstrates that they are not stable association keys.
- Existing SceneProperties Length, Page, Title, and unknown attributes remain retained under their current bounded rules; v3 does not add page/length recalculation or a general SceneProperties editor.
- Unknown or structurally unsupported payloads still must not be guessed away. If a scene cannot be interpreted under the supported derived model, defer that scene's derived update, preserve its source subtree, and continue saving the rest of the document.

This removes the common reopen/stale-arc save failure without adding a private FDX marker, extension, or sidecar.

### Character association and arc-content retention

Use a bounded normalized Character key for association:

1. Read the canonical Character element text without modifying it.
2. Trim outer Unicode whitespace.
3. Remove recognized trailing Character delivery extensions such as (V.O.), (O.S.), (O.C.), and (CONT'D), using the native extension vocabulary rather than stripping arbitrary parenthetical name content.
4. Use a Unicode case-insensitive key for matching and deduplication.
5. Store the derived display name in Unicode uppercase in CharacterArcBeat Name and generated Cast List text.

The real reference demonstrates why the full cue is not the association key: [`real_script_example.fdx`](../../.reference/real_script_example.fdx:13) stores arc Name HELEN MIRREN, while its canonical Character cue at [`real_script_example.fdx`](../../.reference/real_script_example.fdx:23) is HELEN MIRREN (V.O.).

For each scene:

- A matching normalized key retains that Character's existing nested Paragraph and its arc text/runs while updating the derived Name to the canonical uppercase form.
- Adding or deleting other Characters does not alter a retained Character's arc content.
- Renaming a Character to a different normalized identity is a remove-plus-add unless a future explicit character-identity feature says otherwise; arc text does not transfer to an unrelated name merely by position.
- A removed normalized identity loses its derived arc and nested arc text.
- A new normalized identity receives a new derived arc with a new UUID v4 nested Paragraph ID.

Matched nested IDs may remain unchanged as an implementation convenience, but their stability is not a compatibility requirement and they are never ownership proof. There is no need to imitate Final Draft's gratuitous regeneration. Ordinary body and managed title Paragraph ID behavior remains unchanged.

The recognizer must support text-bearing nested arc Paragraphs rather than accepting only empty ones. Arc text is retained by association with the normalized Character key, not by nested UUID and not by arc position.

### Cast List behavior

Cast Lists are also derived presentations:

- Generate their text from the reconciled uppercase scene-arc names, retaining the existing alphabetical case-insensitive ordering and parenthesized formatting.
- Stored derived Cast List text is uppercase even when canonical Character cues are mixed case.
- Preserve the existing paragraph/run container where safely representable, but do not let an inability to correct a formatted derived list block unrelated native saves. If safe correction is unavailable, leave that list unchanged and save the canonical edits.
- Scheduled refresh and undo/redo use the same derived-name and association rules as Save and Save As.

### Residual failure policy

Supported derived data is app-owned and updateable; unsupported derived data is preservable and deferrable. Neither case should make an otherwise valid canonical screenplay document unsaveable.

- Reconcile each scene independently in a detached candidate.
- A supported scene update remains all-or-nothing.
- An unsupported scene retains its complete existing SceneProperties/SceneArcBeats subtree and existing Cast Lists without partial modification.
- Other independent scenes may still update.
- Strict model/preservation validation, managed role/text/source synchronization, serialization/re-read comparison, conflict detection, and atomic replacement remain fatal save requirements.
- Actual invalid candidate state, entropy failure for a required new ID, managed-source mismatch, destination conflict, or storage failure retains existing transactional error behavior.

### Required regression scenarios

- Open an imported document with an existing derived arc, rename/delete its Character after a separate reopen, and save successfully with the corresponding derived arc removed.
- Add and delete other Characters while a retained Character has nonempty arc text; that arc text survives refresh, Save, Save As, and reopen.
- Match mixed-case cues and case-insensitive duplicates while storing uppercase arc Names and Cast List text; canonical cues remain exactly typed.
- Match cues with recognized trailing delivery extensions to extension-free derived arc Names without losing arc content.
- Remove a Character and verify that its derived arc text is removed; add a new Character and verify a fresh arc and UUID are created.
- Mix a supported scene with an unsupported scene; update only the supported scene and save both canonical edits and the unsupported subtree without a document-wide failure.
- Preserve an uncorrectable formatted Cast List while saving unrelated edits.
- Retain fatal behavior for actual managed-source mismatches, invalid candidates, re-read differences, destination conflicts, and storage failures.

## 2. Real-world reference import: repeated opaque title-page IDs

### Confirmed failing construct and source mechanism

[`real_script_example.fdx`](../../.reference/real_script_example.fdx:244) repeats the Paragraph ID ae7f1040-c536-4085-aeec-464de6be3eac across its title-page paragraphs through [`real_script_example.fdx`](../../.reference/real_script_example.fdx:272). These paragraphs have formatting attributes but no Type attribute.

[`collect_identities()`](../../crates/scriptcraft-core/src/fdx.rs:2389) walks every element, maps every Paragraph ID into one global Paragraph identity domain, and returns AmbiguousIdentity when the same domain/value pair is inserted twice at [`fdx.rs`](../../crates/scriptcraft-core/src/fdx.rs:2423). This exactly matches the user's reported incompatible-document/ambiguous-identity failure.

The title-page paragraphs are not managed screenplay elements under [`preserve_node()`](../../crates/scriptcraft-core/src/fdx.rs:2034): management requires a recognized Type role. They instead remain opaque source nodes. Their repeated raw ID attributes can be preserved without pretending that each occurrence is a unique global managed definition.

This is the first confirmed import blocker in the reference. Source inspection alone does not prove that no later unsupported construct will be discovered after this one is fixed.

### Selected strategy: context-scoped identity handling

- Preserve repeated IDs verbatim in opaque title-page Paragraph XML; do not regenerate, deduplicate the actual elements, or remap their source attributes.
- Do not reject those repetitions merely because they share a value in a context where ScriptCraft has no global identity/reference lookup requirement.
- Keep internal source occurrence/order separate from foreign ID values. The existing opaque ordered tree already provides that distinction.
- Continue rejecting duplicate or ambiguous identities where managed body reconciliation, an explicit reference, board/image relationships, or another identity consumer actually requires uniqueness.
- Do not solve the problem by globally disabling duplicate checks or by changing every imported ID to a UUID.

A focused implementation may either avoid registering non-managed repeated title-page Paragraph IDs as global definitions or introduce an explicitly scoped occurrence representation. The work order should choose the smallest change that preserves current strict behavior for managed/reference consumers.

### Required regression scenarios

- Open [`real_script_example.fdx`](../../.reference/real_script_example.fdx:1) successfully, then Save and reopen it without changing canonical body text, title-page text, repeated raw title IDs, board/list data, or opaque sections.
- Use a minimal synthetic fixture with repeated opaque title-page Paragraph IDs and prove element count, order, text, attributes, and raw IDs survive.
- Retain rejection for duplicate managed body IDs and genuinely ambiguous referenced identities.
- After import, reconcile HELEN MIRREN (V.O.) to the existing HELEN MIRREN derived arc using the normalized association rule above.

## 3. Ambiguous imported Content formatting: make whitespace best-effort

### Confirmed source mechanism

For imported documents, [`write_v6()`](../../crates/scriptcraft-core/src/fdx.rs:385) identifies newly inserted managed Paragraph IDs and calls [`format_inserted_paragraph_whitespace()`](../../crates/scriptcraft-core/src/fdx.rs:724).

The formatter is eligible only when Content contains Paragraph elements plus whitespace-only sibling text and no effective XML space preservation applies. [`format_content_insertions()`](../../crates/scriptcraft-core/src/fdx.rs:805) declines mixed/opaque shapes. If any inserted ID remains unformatted, [`format_inserted_paragraph_whitespace()`](../../crates/scriptcraft-core/src/fdx.rs:781) returns an unsupported-construct error.

Thus a valid structural edit can fail to save solely because the serializer could not beautify its sibling gaps. Text-only saves with no newly inserted managed Paragraph do not take this branch, so this mechanism does not explain every existing-file failure.

### Selected strategy

- Keep existing LF/two-space sibling normalization where the unambiguous Content shape permits it.
- When Content is mixed, opaque, or under effective XML space preservation, add no cosmetic sibling whitespace and emit the already reconciled managed Paragraph in its exact source-tree position.
- Remove the document-wide error solely for an unformatted inserted ID. Formatting coverage must not determine whether serialization is legal.
- Continue strict re-read comparison against the actual expected tree. No comparison weakening is needed when expected and emitted trees both retain original sibling nodes plus the managed insertion.
- Continue preserving Text, CDATA, comments, processing instructions, mixed content, unknown elements, source order, and XML space semantics. Do not globally pretty-print imported content.

The result may be less visually attractive XML in an ambiguous imported container, but it is structurally valid and safer than refusing the user's save.

### Required regression scenarios

- Insert and split managed paragraphs under effective XML space preservation, then Save and reopen without changing canonical text or sibling whitespace semantics.
- Repeat with comments, processing instructions, CDATA, non-whitespace mixed text, and non-Paragraph Content children.
- Confirm existing safe formatting still occurs for ordinary Paragraph-plus-structural-whitespace Content.
- Confirm source-tree comparison still rejects unintended opaque-node, identity, role, text, or ordering differences.

## 4. Managed, derived, and preserved FDX boundaries

There are three different questions: what the editor manages, what it derives, and what it merely preserves.

| Layer | ScriptCraft behavior | FDX relationship |
|---|---|---|
| Managed screenplay body | [`Role`](../../crates/scriptcraft-core/src/lib.rs:137) has 17 directed roles; [`ScreenplayElement`](../../crates/scriptcraft-core/src/lib.rs:292) holds role, canonical text, ordered runs, provenance, and a transient internal ID. | These roles map to native Paragraph Type values. Unknown/custom Paragraph types remain source-preserved rather than silently becoming editable roles. |
| New-document generation | [`make_new_document_nodes()`](../../crates/scriptcraft-core/src/fdx.rs:1095) emits Content, 17 ElementSettings blocks, optional TitlePage, and SceneProperties/SceneArcBeats for headings. | These are native FDX structures. The minimal subset, canonical order, and whitespace convention are ScriptCraft generation policy, not a requirement for every valid FDX file. |
| Derived scene data | Supported SceneProperties scene-cast arcs are rebuilt from normalized current Character identities. Retained Characters keep arc content; removed Characters lose derived arcs. Stored derived names are uppercase. | The elements are FDX-native, but their supported scene-cast projection is app-owned after import. Nested derived IDs are not durable ownership evidence. |
| Derived Cast Lists | Current scene-arc names produce uppercase alphabetized list text. | Native Cast List Paragraphs remain in the FDX body, but their generated text is derived rather than canonical Character content. |
| Runtime-only state | Internal StableId values, selection, undo history, dirty state, layout caches, and editing policy remain session state. | They intentionally have no durable FDX representation. |
| Imported settings | Runtime layout/editing interprets only bounded supported fields and shared application defaults. Imported Behavior and unsupported values remain source-owned. | FDX can carry settings ScriptCraft does not execute or render exactly. Preservation does not imply editing or layout parity. |
| Other FDX sections and payloads | Unknown top-level sections, SmartType, board/list/image graphs, title header/footer structure, comments, namespaces, and opaque nested content are retained. | FDX is broader than ScriptCraft's editing UI. These structures need not gain semantic app equivalents merely to round-trip safely. |
| XML lexical form | ScriptCraft writes one declaration and deterministic formatting for generated structure; imported trees are serialized structurally. | Quote style, entity spelling, attribute interleaving, and unrelated whitespace are not universal byte-identity guarantees. |

### Target invariants

1. Managed role, text, order, ordinary screenplay IDs, and supported source mappings reconcile exactly or the specific unsafe edit fails transactionally.
2. Supported derived scene/cast data is app-owned after import and follows current Character edits using normalized Character association.
3. Unsupported or unknown FDX content remains preserved and does not become editable by implication.
4. Unsupported derived updates and cosmetic formatting may be deferred; neither holds canonical screenplay saves hostage.
5. No private ScriptCraft ownership/schema extension is needed.

## Proposed v3 work-order boundaries

These are planning boundaries only, pending user approval. Execute sequentially under the repository's default policy.

### [V3-WO-001.1 — Derived Character name projection](work-orders-v3/WO-001.1-derived-character-projection.md)

Define normalized association keys and uppercase stored derived names, including recognized trailing cue-extension removal. Wire the projection into new-document generation first without changing imported arc ownership or Cast List behavior.

### [V3-WO-001.2 — App-owned scene-arc membership after import](work-orders-v3/WO-001.2-app-owned-scene-arcs.md)

Use the normalized projection for imported scenes, remove runtime creation-proof deletion gating, and support post-reopen Character add/remove/rename with successful Save/Save As. Keep text-bearing arc support and unsupported-scene deferral out of this order.

### [V3-WO-001.3 — Scene-arc content retention by Character association](work-orders-v3/WO-001.3-scene-arc-content-retention.md)

Support text-bearing arc Paragraphs. Preserve a retained Character's arc content by normalized name across unrelated roster edits, and remove derived arc content when its Character is removed or renamed to a different identity.

### [V3-WO-001.4 — Nonblocking derived refresh, Cast Lists, and native save](work-orders-v3/WO-001.4-nonblocking-derived-save.md)

Add per-scene deferral for unsupported derived shapes, preserve uncorrectable formatted Cast Lists without blocking save, and generate uppercase Cast List text from fully reconciled scene projections across refresh, history, Save, and Save As.

### [V3-WO-002 — Context-scoped identities for opaque title-page Paragraphs](work-orders-v3/WO-002-context-scoped-title-page-identities.md)

Permit repeated IDs in opaque title-page Paragraphs while preserving raw source attributes and strict managed/reference identity checks. Add the real-reference open/save/reopen scenario and focused duplicate-opaque versus duplicate-managed fixtures.

### [V3-WO-003 — Best-effort imported insertion formatting](work-orders-v3/WO-003-best-effort-insertion-formatting.md)

Keep safe normalization for unambiguous Content and skip it otherwise. Remove formatting-only unsupported-construct failures while preserving strict source-tree re-read validation and add mixed-content, XML-space, split/insert, and save/reopen cases.

## Durable documents requiring amendment during implementation

- [`screenplay-writing-loop.md`](../../.forge/requirements/features/screenplay-writing-loop.md:138): replace creation-ownership deletion rules and first-spelling derived names; define normalized uppercase derived data, arc-text retention/removal, repeated opaque title-page IDs, derived-update deferral, and best-effort formatting.
- [`product-overview.md`](../../.forge/requirements/product-overview.md:20): distinguish app-owned supported derived scene/cast data from preserved unknown FDX content and canonical mixed-case screenplay text.
- [`technical-context.md`](../../.forge/blueprints/technical-context.md:43): remove runtime ownership-proof and document-wide derived-stop contracts; document normalized Character association, scoped identity handling, and nonfatal derived/formatting deferral.

No work order should change ordinary screenplay UUID behavior, redesign the status bar, regenerate all nested IDs merely to imitate Final Draft, broaden Scene Heading structural editing, or claim blanket Final Draft compatibility.

## Execution log — V3-WO-001.1: Derived Character name projection

- **Status:** Implementation authored; review and validation remain user-owned under repository policy.
- **Changes:** Added a bounded new-document Character projection that trims Unicode whitespace, removes at most one recognized trailing delivery extension, ignores empty projected cues, deduplicates in first-occurrence order by Unicode lowercase key, and stores the derived name in Unicode uppercase. New-document SceneArcBeats generation now uses it. Canonical Character text is untouched. Imported reconciliation and Cast List consumers remain on their prior paths, as required by this order's sequencing boundary.
- **Regression coverage authored:** Added `crates/scriptcraft-core/tests/v3_wo0011_derived_character_projection.rs` cases covering mixed-case duplicates, extension-free matching, all recognized suffixes and case variants, Unicode whitespace, empty cues, arbitrary parentheticals, order, uppercase expansion (`ß` to `SS`), and preservation of canonical cue text.
- **Durable documentation:** Updated the derived-name contract in `.forge/requirements/features/screenplay-writing-loop.md` and `.forge/blueprints/technical-context.md`, explicitly limiting the new behavior to new-document generation.
- **Validation:** No tests, builds, formatting, lint, or other checks were run. The work order assigns checks to the user, and the repository-wide policy prohibits agent-run validation/review.
- **Limitations/blockers:** Test cases are authored but unexecuted and therefore not confirmed to compile or pass. Imported-scene reconciliation, Cast List consumption, arc ownership/content handling, and save behavior are intentionally unchanged and remain for later work orders.

## Execution log — V3-WO-001.2: App-owned scene-arc membership after import

- **Status:** Implementation and focused regression authoring recorded; user review and validation remain pending under the authoritative repository policy.
- **Changes:** Imported scene reconciliation now consumes the shared normalized Character projection from V3-WO-001.1: trims cues, removes one recognized delivery extension, deduplicates by normalized key, stores uppercase names, and keeps canonical cue text untouched. Matched supported arcs associate by normalized name and retain their nested Paragraph IDs; stale supported entries can be removed after unique-ID/reference-safety checks without ScriptCraft creation history; new or differently renamed identities receive bounded UUID v4 allocations. The runtime generated-arc ownership set, import/new-document initializers, writer-re-read propagation helper/call, and associated creation-proof expectations were removed. Strict detached candidate validation, source-tree serialization/re-read comparison, conflict handling, and atomic publication paths were not intentionally changed. Text-bearing and other unsupported arc shapes still safe-stop in this order.
- **Changed files:** [`scene_metadata.rs`](../../crates/scriptcraft-core/src/scene_metadata.rs), [`preservation.rs`](../../crates/scriptcraft-core/src/preservation.rs), [`editing.rs`](../../crates/scriptcraft-core/src/editing.rs), [`fdx.rs`](../../crates/scriptcraft-core/src/fdx.rs), [`fdx_storage.rs`](../../crates/scriptcraft-core/src/fdx_storage.rs), [`cast_list.rs`](../../crates/scriptcraft-core/src/cast_list.rs), [`v3_wo0012_app_owned_scene_arcs.rs`](../../crates/scriptcraft-core/tests/v3_wo0012_app_owned_scene_arcs.rs), [`wo0032_uuid_lifecycle.rs`](../../crates/scriptcraft-core/tests/wo0032_uuid_lifecycle.rs), [`screenplay-writing-loop.md`](../../.forge/requirements/features/screenplay-writing-loop.md), [`product-overview.md`](../../.forge/requirements/product-overview.md), [`technical-context.md`](../../.forge/blueprints/technical-context.md), and this execution log. The V3-WO-001.1 implementation was retained and reused without reverting its new-document projection.
- **Regression coverage authored:** Added [`v3_wo0012_app_owned_scene_arcs.rs`](../../crates/scriptcraft-core/tests/v3_wo0012_app_owned_scene_arcs.rs) for post-reopen delete followed by Save and Save As, normalized-name association across add/remove/reorder and rename, uppercase derived Names with unchanged canonical mixed-case cues, delivery-extension-free arc matching, undo/redo state restoration, fatal source mismatch, and existing-destination Save As failure. Replaced the contradictory stale-arc expectations in [`scene_metadata.rs`](../../crates/scriptcraft-core/src/scene_metadata.rs), [`cast_list.rs`](../../crates/scriptcraft-core/src/cast_list.rs), and [`wo0032_uuid_lifecycle.rs`](../../crates/scriptcraft-core/tests/wo0032_uuid_lifecycle.rs). Existing storage failure and managed-source safeguards remain represented in their existing focused tests.
- **Durable documentation:** Updated [`screenplay-writing-loop.md`](../../.forge/requirements/features/screenplay-writing-loop.md), [`product-overview.md`](../../.forge/requirements/product-overview.md), and [`technical-context.md`](../../.forge/blueprints/technical-context.md) to define app-owned membership for the recognized supported empty nested-Paragraph arc shape, normalized-name matching, uppercase derived names, safe removal without creation proof, and the unsupported-content boundary.
- **Validation performed or omitted:** No tests, builds, formatting, lint, diff checks, manual verification, implementation checks, or reviews were run. Repository policy assigns all review and validation to the user and prohibits agent-run checks. Regression tests are authored artifacts only, not passing results.
- **Limitations/blockers:** Nonempty/text-bearing nested arc Paragraphs remain unsupported and can still stop document-wide reconciliation; preserving that content is V3-WO-001.3, while per-scene deferral and nonblocking derived saves are V3-WO-001.4. Cast List formatting/nonblocking behavior remains outside this order. No compile, test, save, or interoperability result is claimed; user-owned review and validation remain outstanding.

## Execution log — V3-WO-001.3: Scene-arc content retention by Character association

- **Status:** Implementation and focused regression authoring completed; review and validation remain user-owned under the authoritative repository policy.
- **Changes:** Broadened the bounded `CharacterArcBeat` recognizer to accept exactly one same-namespace nested `Paragraph` with source attributes and ordered same-namespace `Text` runs containing text or CDATA, optionally separated by whitespace-only formatting nodes. The arc Paragraph remains opaque to semantic screenplay content. Matched normalized Character keys retain their entire original nested Paragraph subtree, including its ID, text, run order, run formatting, and paragraph/run attributes; reconciliation continues to update only derived beat Name and ordering. Unknown non-text child structures, duplicate normalized beats, multiple nested Paragraphs, and unsafe identity/reference cases continue to safe-stop rather than guess. Removal deletes the derived arc subtree and identity record; a different normalized key is a remove-plus-new-blank arc and receives a fresh allocated ID. The existing source-mismatch, save transaction, and UUID allocation paths were not intentionally changed.
- **Changed files:** [`scene_metadata.rs`](../../crates/scriptcraft-core/src/scene_metadata.rs), [`v3_wo0013_scene_arc_content_retention.rs`](../../crates/scriptcraft-core/tests/v3_wo0013_scene_arc_content_retention.rs), [`screenplay-writing-loop.md`](../../.forge/requirements/features/screenplay-writing-loop.md), [`product-overview.md`](../../.forge/requirements/product-overview.md), [`technical-context.md`](../../.forge/blueprints/technical-context.md), and this execution log. Updated the prior unit case to continue asserting atomic rejection of an unsupported non-text arc child.
- **Regression coverage authored:** Added cases for nonempty ordered run content/attributes and exact save/reopen subtree retention; unrelated Character addition, removal, and reordering; delivery-extension and case-insensitive association; removal of derived arc text; rename without content transfer; independent same-name arc content across multiple scenes; duplicate beats and multiple nested Paragraph rejection; and undo/redo plus Save and Save As retention. These are authored cases, not validation results.
- **Durable documentation:** Updated the supported arc-content contract in [`screenplay-writing-loop.md`](../../.forge/requirements/features/screenplay-writing-loop.md), [`product-overview.md`](../../.forge/requirements/product-overview.md), and [`technical-context.md`](../../.forge/blueprints/technical-context.md): retained Characters keep supported arc content by normalized association, while removed Characters lose that derived content and renames do not transfer it.
- **Validation performed or omitted:** No tests, builds, formatting, lint, diff checks, manual verification, implementation checks, or reviews were run. The repository-wide policy assigns all review and validation to the user and prohibits agent-run checks. No compile, test, save, or interoperability result is claimed.
- **Limitations/blockers:** Authored regression coverage is unexecuted and not confirmed to compile or pass. Ambiguous/unsupported arc shapes still stop reconciliation atomically and may block document save; per-scene deferral and nonblocking derived save handling remain deferred to V3-WO-001.4. User-owned review and validation remain outstanding.

## Execution log — V3-WO-001.4: Nonblocking derived refresh, Cast Lists, and native save

- **Status:** Implementation and focused regression authoring recorded; user review and validation remain pending under the authoritative repository policy.
- **Changes:** Scene reconciliation now runs each managed heading against an isolated preservation-state candidate. `UnsupportedSceneMetadata` discards only that heading's attempted changes and records it as deferred; independent supported headings continue, while model validation, preservation errors, required-ID allocation failures, and later serialization/re-read, conflict, and storage failures retain their fatal behavior. Cast List correction skips lists belonging to deferred scenes, consumes the uppercase reconciled arc projection for source-backed scenes, uses the same uppercase normalized Character projection for preservation-free projects, and leaves an uncorrectable formatted list's text/runs unchanged while updating other safe lists. These policies flow through scheduled refresh, undo/redo correction, Save, and Save As using the existing detached-candidate and atomic publication paths; expected scene deferral therefore does not set the recurring desktop refresh error.
- **Changed files:** [`scene_metadata.rs`](../../crates/scriptcraft-core/src/scene_metadata.rs), [`cast_list.rs`](../../crates/scriptcraft-core/src/cast_list.rs), [`v3_wo0014_nonblocking_derived_save.rs`](../../crates/scriptcraft-core/tests/v3_wo0014_nonblocking_derived_save.rs), [`v3_wo0013_scene_arc_content_retention.rs`](../../crates/scriptcraft-core/tests/v3_wo0013_scene_arc_content_retention.rs), [`main.rs`](../../apps/scriptcraft/src/main.rs), [`screenplay-writing-loop.md`](../../.forge/requirements/features/screenplay-writing-loop.md), [`product-overview.md`](../../.forge/requirements/product-overview.md), [`technical-context.md`](../../.forge/blueprints/technical-context.md), and this execution log. Prior WO-001.1 through WO-001.3 behavior and artifacts were retained; no later work order or unrelated feature was implemented.
- **Regression coverage authored:** Added a core mixed-scene scenario that expects supported scene metadata/Cast Lists to update while an unsupported scene subtree and its lists remain unchanged, canonical edits to succeed through Save and Save As, and uncorrectable formatted-list text and run attributes to survive. Additional authored cases cover deferred scenes during refresh and undo/redo, uppercase stored Cast List text for source-backed and preservation-free projects, and repeated desktop scheduled refresh without surfacing a save-style error. Updated prior WO-001.3 unsupported-shape coverage to assert preservation/deferral rather than the superseded whole-operation error. Existing fatal-path coverage for invalid or mismatched candidates, required identity allocation, re-read differences, destination conflicts, and storage failures was not weakened.
- **Durable documentation:** Updated [`screenplay-writing-loop.md`](../../.forge/requirements/features/screenplay-writing-loop.md), [`product-overview.md`](../../.forge/requirements/product-overview.md), and [`technical-context.md`](../../.forge/blueprints/technical-context.md) to state that safe-stop applies to the derived update rather than an otherwise valid canonical native save, that unsupported scenes and uncorrectable formatted Cast Lists remain unchanged, and that stored derived Cast List names are uppercase.
- **Validation performed or omitted:** No tests, builds, formatting, lint, diff checks, manual verification, implementation checks, reviews, or interoperability checks were run. The repository-wide user-owned validation policy prohibits agent-run checks. Regression coverage is authored but unexecuted; no compile, test, save, or runtime pass is claimed.
- **Limitations/blockers:** User review and all validation remain outstanding. Authored regression cases are unconfirmed to compile or pass. Unsupported scene data and formatted Cast Lists intentionally remain stale until they can be safely corrected; this limitation does not block canonical native saves. No changes were made to subsequent title-page identity or insertion-formatting work orders.

## Execution log — V3-WO-002: Context-scoped identities for opaque title-page Paragraphs

- **Status:** Implementation and focused regression authoring recorded; review and validation remain user-owned under the repository-wide policy.
- **Changes:** Made Paragraph identity collection aware of TitlePage ancestry and the direct recognized title-role boundary used by `preserve_node()`. Opaque TitlePage Paragraph IDs are counted as source occurrences. A single such occurrence retains its existing global Paragraph identity registration; when multiple opaque occurrences share an ID, none is registered as a unique global definition, while each complete Paragraph remains in the ordered opaque source tree with its original ID, attributes, text/runs, and position. Repeated opaque IDs cannot shadow any globally registered Paragraph identity. Managed body/title identities, duplicate-attribute rejection, and existing board/item/image identity/reference validation remain strict. The chosen smallest representation is the existing ordered opaque preservation tree; no occurrence identities, private FDX fields, or source ID rewrites were added. The preceding V3-WO-001.1–001.4 behavior was left intact.
- **Changed files:** [`fdx.rs`](../../crates/scriptcraft-core/src/fdx.rs), [`v3_wo002_context_scoped_title_page_identities.rs`](../../crates/scriptcraft-core/tests/v3_wo002_context_scoped_title_page_identities.rs), [`screenplay-writing-loop.md`](../../.forge/requirements/features/screenplay-writing-loop.md), [`product-overview.md`](../../.forge/requirements/product-overview.md), [`technical-context.md`](../../.forge/blueprints/technical-context.md), and this execution log.
- **Regression coverage authored:** Added minimal repeated opaque TitlePage ID Save/reopen coverage asserting paragraph count/order, full attributes, text/runs, raw IDs, canonical body, and preservation/global registration of distinct opaque and managed title IDs. Added real-reference in-memory open/serialize/reopen coverage for body text, title-page Paragraph content/IDs, board graph/references, and opaque top-level sections, plus normalized association of `HELEN MIRREN (V.O.)` with the retained `HELEN MIRREN` derived arc. Added rejection cases for duplicate managed body IDs, duplicate referenced ListItem identities, and opaque repeated IDs that collide with a globally registered managed Paragraph. These are authored cases, not execution results.
- **Durable documentation:** Updated [`screenplay-writing-loop.md`](../../.forge/requirements/features/screenplay-writing-loop.md), [`product-overview.md`](../../.forge/requirements/product-overview.md), and [`technical-context.md`](../../.forge/blueprints/technical-context.md) to distinguish repeated raw IDs in non-managed opaque TitlePage occurrences from globally unique managed/reference identities, while retaining the existing treatment of a distinct opaque occurrence.
- **Validation performed or omitted:** No tests, builds, formatting, lint, diff inspection/checks, manual verification, implementation checks, or reviews were run. The authoritative repository policy assigns all review and validation to the user and prohibits agent-run checks. The new regression cases are unexecuted; no compile, open/save, or interoperability pass is claimed.
- **Limitations/blockers:** User review and validation remain outstanding. In particular, the real-reference open/save/reopen and derived-association scenarios are authored but unverified; any later independent blocker in that document remains unknown until user validation. No work from V3-WO-003 or unrelated scope was implemented.

## Execution log — V3-WO-003: Best-effort imported insertion formatting

- **Status:** Implementation and focused regression authoring recorded; review and validation remain user-owned under the authoritative repository policy.
- **Changes:** Changed imported inserted/split Paragraph sibling formatting from a serialization precondition to a best-effort normalization. The existing LF/four-space Content sibling normalization (two-space indentation beneath the root plus Content depth) remains limited to unambiguous Paragraph-plus-whitespace Content without effective `xml:space="preserve"`; mixed/opaque Content and preserved-space contexts retain their original sibling nodes and receive no cosmetic whitespace. Removed the formatted-ID coverage refusal and its bookkeeping while leaving source reconciliation, detached expected-tree construction, identity/semantic validation, strict expected-versus-re-read source-tree comparison, and generated-document formatting policy unchanged. The WO-001.1–001.4 and WO-002 implementation remains intact.
- **Changed files:** [`fdx.rs`](../../crates/scriptcraft-core/src/fdx.rs), [`wo0042_fdx_structural_whitespace.rs`](../../crates/scriptcraft-core/tests/wo0042_fdx_structural_whitespace.rs), [`v3_wo003_best_effort_insertion_formatting.rs`](../../crates/scriptcraft-core/tests/v3_wo003_best_effort_insertion_formatting.rs), [`screenplay-writing-loop.md`](../../.forge/requirements/features/screenplay-writing-loop.md), [`technical-context.md`](../../.forge/blueprints/technical-context.md), and this execution log.
- **Regression coverage authored:** Replaced the previous xml:space/mixed-Content formatting refusal expectations with successful serialization and reopen assertions. Added focused coverage for insertion and split under effective `xml:space="preserve"`, retaining direct sibling nodes and canonical text through reopen; split in Content containing comments, a processing instruction, CDATA, non-whitespace mixed text, and a non-Paragraph element; the safe normalization path for unambiguous Paragraph-only Content; and continued rejection of an unanchored managed source insertion. Existing generated-document declaration/section/indentation/final-LF and canonical tab/LF/CR cases remain in the revised structural-whitespace suite. The writer's strict semantic/source comparison code was not modified.
- **Durable documentation:** Updated the imported formatting requirement in [`screenplay-writing-loop.md`](../../.forge/requirements/features/screenplay-writing-loop.md) and formatting contract in [`technical-context.md`](../../.forge/blueprints/technical-context.md): safe sibling formatting is best-effort, and ambiguous or `xml:space`-preserving contexts emit the reconciled Paragraph at its source-tree position without cosmetic normalization rather than refusing an otherwise valid save.
- **Validation performed or omitted:** No tests, builds, formatting, lint, diff checks, manual verification, implementation checks, or reviews were run. The repository-wide policy assigns all review and validation to the user and prohibits agent-run checks. The new and revised regression cases are authored but unexecuted; no compile, Save/Save As, reopen, or interoperability pass is claimed.
- **Limitations/blockers:** User review and all validation remain outstanding; test compilation and runtime behavior are unconfirmed. Regression save/reopen coverage exercises the core `write_v6()` serialization/re-read path in memory rather than filesystem Save/Save As transactions. The broader strict re-read checks remain implemented but are not newly exhaustively exercised by this work order. No unrelated changes were made.

## Final handoff — user-owned checks, unverified items, and deferred limitations

Repository policy assigns all review and validation to the user. No checks below have been run for these six v3 work orders; the implementation logs record authored regression cases, not passing results. Nothing in this handoff establishes that the code compiles or that the reported behaviors work at runtime.

### Suggested user-run command checks

Run the repository's applicable Cargo checks from the workspace root:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
cargo test --workspace
```

The following focused regression targets are new or were materially revised by this work. Run them individually to localize failures, then run the full workspace suite above:

```sh
cargo test -p scriptcraft-core --test v3_wo0011_derived_character_projection
cargo test -p scriptcraft-core --test v3_wo0012_app_owned_scene_arcs
cargo test -p scriptcraft-core --test v3_wo0013_scene_arc_content_retention
cargo test -p scriptcraft-core --test v3_wo0014_nonblocking_derived_save
cargo test -p scriptcraft-core --test v3_wo002_context_scoped_title_page_identities
cargo test -p scriptcraft-core --test v3_wo003_best_effort_insertion_formatting
cargo test -p scriptcraft-core --test wo0032_uuid_lifecycle
cargo test -p scriptcraft-core --test wo0042_fdx_structural_whitespace
```

The corresponding test sources are [`v3_wo0011_derived_character_projection.rs`](../../crates/scriptcraft-core/tests/v3_wo0011_derived_character_projection.rs:1), [`v3_wo0012_app_owned_scene_arcs.rs`](../../crates/scriptcraft-core/tests/v3_wo0012_app_owned_scene_arcs.rs:1), [`v3_wo0013_scene_arc_content_retention.rs`](../../crates/scriptcraft-core/tests/v3_wo0013_scene_arc_content_retention.rs:1), [`v3_wo0014_nonblocking_derived_save.rs`](../../crates/scriptcraft-core/tests/v3_wo0014_nonblocking_derived_save.rs:1), [`v3_wo002_context_scoped_title_page_identities.rs`](../../crates/scriptcraft-core/tests/v3_wo002_context_scoped_title_page_identities.rs:1), [`v3_wo003_best_effort_insertion_formatting.rs`](../../crates/scriptcraft-core/tests/v3_wo003_best_effort_insertion_formatting.rs:1), [`wo0032_uuid_lifecycle.rs`](../../crates/scriptcraft-core/tests/wo0032_uuid_lifecycle.rs:1), and [`wo0042_fdx_structural_whitespace.rs`](../../crates/scriptcraft-core/tests/wo0042_fdx_structural_whitespace.rs:1).

### Desktop and filesystem Save/Save As/reopen checks

After the command checks, launch the desktop app with `cargo run -p scriptcraft` and use disposable copies of documents. Exercise both Save to the current file and Save As to a distinct path, close the document/app, then reopen each saved file. Verify that successful edits persist, failed saves leave the original intact, and Save As does not alter its source. Include at least:

- An imported document edited after reopening, with Character add/remove/rename and retained nonempty scene-arc text; inspect uppercase derived arc/Cast List names while confirming canonical Character cue case remains unchanged.
- A document with a supported scene alongside an unsupported scene shape and a formatted/stale Cast List; confirm the supported projection updates while deferred scene/list source content remains unchanged and canonical edits still save.
- Structural insertion and split in ordinary imported Content and in mixed-content and effective `xml:space="preserve"` contexts; reopen and check canonical text, sibling whitespace semantics, and preservation of comments, processing instructions, CDATA, and unknown children.
- Save As to a new destination and the existing-destination/conflict and storage-failure paths where safely reproducible, confirming transactional failure does not publish a partial file or corrupt the source.

**Important coverage gap:** V3-WO-003's authored Save/reopen scenarios call the core `write_v6()` serialization and re-read path in memory. They do not exercise filesystem Save/Save As transactions. The desktop/file-system checks above remain necessary; do not interpret the in-memory cases as evidence of filesystem transaction coverage.

### Real-reference and Final Draft interoperability checks

- Use a disposable copy of [`real_script_example.fdx`](../../.reference/real_script_example.fdx:1), not the checked-in reference itself. Open it in ScriptCraft, verify the repeated opaque title-page Paragraph IDs and content survive, Save and Save As to disposable paths, close/reopen those outputs, and inspect body/title-page content, board/list references, opaque sections, and the `HELEN MIRREN (V.O.)` to `HELEN MIRREN` derived association. The authored real-reference scenario is in-memory; an app/file-system run is still outstanding. This is the first known import blocker addressed, not proof that the reference has no other independent blocker.
- Where Final Draft is available, open a ScriptCraft-saved disposable FDX in Final Draft, inspect representative body, title-page, scene-arc, Cast List, and preserved board/list data, save a separate copy from Final Draft, and reopen that copy in ScriptCraft. Record the Final Draft version and any transformations. This is targeted interoperability checking only and must not be generalized into a blanket compatibility claim.

### Outstanding verification versus known defects

- **Not yet run / unverified:** All new and revised regression suites; compilation, formatting, lint, and build checks; code/repository review; desktop launch and scheduled refresh behavior; real-reference import through the desktop; actual filesystem Save, Save As, conflict/failure, close/reopen; and Final Draft application interoperability. Test compilation and runtime behavior are unknown. These are verification gaps, not confirmed defects.
- **Known defect confirmed by the prior investigation, with a fix authored but unverified here:** The real reference previously failed import because opaque title-page Paragraph IDs repeat. The v3 change is intended to scope that identity handling; no post-change successful import has been established. Any later independent blocker in the reference remains unknown until the user runs the check.
- **Known intentional limitations/deferred behavior:** Unsupported scene-arc shapes are deferred per scene and their subtrees are preserved rather than reconciled; Cast Lists for deferred scenes and formatted lists that cannot safely be rewritten may consequently remain stale. Arbitrary arc-payload editing, beat-board editing, broad diagnostics for deferred projections, SceneProperties Page/Length recalculation, and Scene Heading structural split/join support are outside these work orders. Ambiguous imported Content receives no cosmetic whitespace normalization, and imported XML lexical byte identity is not promised. PDF non-ASCII substitution and broad Final Draft compatibility remain separate known limitations. Opaque title-page content is preserved, not made editable.
- **No new actual defect is confirmed by this work record.** Any failures discovered by user-run checks should be recorded separately with the exact command or reproduction and affected document; do not relabel an unrun check as a failure or a pass.

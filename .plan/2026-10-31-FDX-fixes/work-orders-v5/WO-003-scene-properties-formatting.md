# V5-WO-003 — SceneProperties structural formatting and deferred nested-ID regeneration

- **Status:** Implemented; regressions authored but unexecuted; user validation pending
- **Depends on:** V5-WO-002 declaration whitespace fixed-point repair
- **Validation:** User-owned; no agent-run checks or independent validators

## Objective

Format supported generated SceneProperties/SceneArcBeats XML with the same deterministic LF and two-space indentation convention used for other generated ScriptCraft structure, while preserving canonical arc content, IDs, source attributes, unsupported subtrees, and strict round-trip checks.

Record Final Draft's regeneration of nested scene-arc Paragraph UUIDs as a deferred compatibility observation. Do not imitate it.

## Current behavior and target

V4-WO-002 regenerates supported containers in [`reconcile_heading()`](../../../crates/scriptcraft-core/src/scene_metadata.rs:206). The generated container from [`generated_scene_properties()`](../../../crates/scriptcraft-core/src/scene_metadata.rs:388) does not itself add structural whitespace, and newly inserted/reordered arc entries can consequently appear adjacent rather than using the reference indentation.

Final Draft's populated scene example in [`reference2.fdx`](../../../.reference/reference2.fdx:40) uses:

- SceneProperties indented one level beneath its Scene Heading;
- SceneArcBeats indented one level beneath SceneProperties;
- each CharacterArcBeat indented one level beneath SceneArcBeats;
- each nested arc Paragraph indented one level beneath its CharacterArcBeat;
- closing tags aligned with their corresponding opening elements;
- no Text child manufactured for an empty arc Paragraph.

New preservation-free generation already has recursive element-only formatting in [`format_generated_element()`](../../../crates/scriptcraft-core/src/fdx.rs). Supported imported/generated SceneProperties should follow that same structural convention at the correct managed-heading depth.

## Scope

1. Apply deterministic structural formatting to supported generated SceneProperties containers during source-backed reconciliation/serialization.
2. Use LF and two ASCII spaces per depth relative to the managed Scene Heading and its ancestors.
3. Include the same formatting in the detached expected source tree before strict re-read comparison.
4. Preserve retained CharacterArcBeat subtrees, normalized-name associations, nested IDs, attributes, arc text/runs, and ambiguous-scene deferral.
5. Add focused direct-codec and filesystem Save/Save As/reopen formatting regressions.
6. Record the nested UUID regeneration difference as a future compatibility TODO without changing current identity behavior.

## Non-goals

- No regeneration of matched nested scene-arc Paragraph UUIDs to imitate Final Draft.
- No global pretty-printing of imported FDX or opaque content.
- No formatting of deferred/unsupported SceneProperties subtrees.
- No insertion of whitespace into canonical screenplay Text, arc Text, CDATA, comments, processing instructions, or mixed content.
- No SceneProperties Page/Length recalculation, arc editor, or ownership redesign.
- No changes to declaration separation, identity checks, semantic checks, conflict detection, or atomic-save behavior.
- No agent-run tests, desktop automation, Final Draft automation, review, or validation.

## Implementation requirements

### 1. Format only supported generated containers

Use the existing supported-scene reconciliation boundary to distinguish containers ScriptCraft regenerates from deferred source subtrees. Apply formatting only after the complete supported scene candidate has been validated and only to the generated SceneProperties/SceneArcBeats structure.

Reuse the existing generated-formatting semantics where practical, or introduce a small shared bounded helper with equivalent behavior. The formatting path must know the actual managed-heading depth rather than treating SceneProperties as a document root.

Do not infer that every SceneProperties element is generated. Deferred or ambiguous scenes must retain their complete original subtree and whitespace.

### 2. Match native structural indentation

For a standard body Scene Heading nested under FinalDraft/Content, formatting should produce the depth demonstrated in [`reference2.fdx`](../../../.reference/reference2.fdx:40): SceneProperties one level below the heading, SceneArcBeats one level below that, CharacterArcBeat entries one level lower, and nested Paragraphs one level lower again.

For namespaced or otherwise nested valid source, compute depth from the actual preserved tree rather than hard-coding spaces. Respect effective xml:space preservation: skip cosmetic formatting when preservation semantics make added whitespace unsafe or ambiguous.

An empty SceneArcBeats may remain self-closing as in [`reference2.fdx`](../../../.reference/reference2.fdx:87). Do not add child whitespace to an element with no children.

### 3. Preserve arc content and opaque nodes

Formatting may add structural text nodes only in element-only generated containers where those nodes are cosmetic and included in the expected tree.

Do not alter:

- CharacterArcBeat Name association or normalized order;
- nested Paragraph IDs or other attributes;
- ordered arc Text runs, arc text, run formatting, or CDATA;
- opaque children within a retained arc Paragraph;
- unsupported scene payloads;
- canonical screenplay text or paragraph-level source attributes.

A text-bearing arc Paragraph must not receive pretty-print whitespace among its Text runs. Existing meaningful mixed content remains untouched.

### 4. Keep strict source-tree comparison synchronized

Apply formatting before the expected sections are cloned for output comparison in [`write_v6_inner()`](../../../crates/scriptcraft-core/src/fdx.rs:514), or otherwise ensure the candidate and expected tree contain exactly the same structural whitespace before serialization.

Do not special-case SceneProperties whitespace in code 30 comparison. The emitted bytes and expected tree must agree under the existing normalization rules, and repeated saves must be fixed-point stable.

### 5. Defer native nested UUID regeneration

Do not implement Final Draft's regenerate-on-every-save behavior. Current normalized Character names are the association keys; nested UUIDs are source identities retained for matched Characters. Regenerating them would add file churn and reference risk without improving the current model's association semantics.

Add a future compatibility TODO to [`technical-context.md`](../../../.forge/blueprints/technical-context.md) stating:

- Final Draft is observed to regenerate nested scene-arc Paragraph UUIDs on every save, even without edits;
- ScriptCraft intentionally retains matched nested IDs and allocates only for new derived entries;
- this is not currently evidence of a compatibility defect;
- revisit only if a concrete Final Draft interoperability scenario demonstrates that regeneration is required for semantics, references, or reopen behavior.

No additional UUID work order is approved.

## Regression coverage

Author [`v5_wo003_scene_properties_formatting.rs`](../../../crates/scriptcraft-core/tests/v5_wo003_scene_properties_formatting.rs).

Cover:

1. A supported managed heading with no existing SceneProperties receives correctly indented generated SceneProperties and SceneArcBeats before its first managed Text child.
2. A supported scene with multiple Characters emits each CharacterArcBeat and nested Paragraph on separate correctly indented lines with aligned closing tags.
3. An empty SceneArcBeats remains self-closing without manufactured child whitespace.
4. JOHN/Arc 1 from [`arc copy.fdx`](../../../.reference/arc%20copy.fdx:15) retains its text, Paragraph/Text attributes, normalized-name association, and nested ID while surrounding generated structure is formatted.
5. Character addition/deletion/reordering changes only derived membership/order plus structural formatting; retained arc content remains exact.
6. New arc entries receive fresh IDs and existing matched IDs remain unchanged across Save, reopen, Save As, and repeated Save.
7. Text-bearing arc Paragraphs receive no cosmetic whitespace among their runs; CDATA/comments/PIs/mixed content controls remain unchanged.
8. Deferred unsupported SceneProperties retains its complete original subtree and whitespace while an independent supported scene is formatted.
9. Effective xml:space preservation skips cosmetic formatting without blocking Save.
10. Direct serialization and full filesystem Save/Save As/reopen/repeated Save remain code-30-free and fixed-point stable for the references.
11. New preservation-free generated formatting remains unchanged.
12. An independently injected genuine source-tree mismatch still fails with code 30.

Assert structural whitespace and content/identity separately. Do not require Final Draft's nested UUID regeneration and do not assert whole-file byte identity with Final Draft.

## Durable documentation

Amend:

- [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md:154) with bounded supported SceneProperties formatting, correct-depth LF/two-space behavior, mixed-content/xml:space exclusions, and strict expected-tree synchronization.
- [`technical-context.md`](../../../.forge/blueprints/technical-context.md:45) with the supported-container formatting boundary and the deferred nested-UUID-regeneration TODO described above.
- [`fdx-fixes-v5.md`](../fdx-fixes-v5.md) with implementation/authored-test evidence and the explicit decision not to imitate native ID regeneration.

No product-overview change is required unless existing wording implies gratuitous nested-ID regeneration; current normalized-name association should remain intact.

## User-owned validation

Run applicable formatting, lint, build, workspace, and focused V5-WO-003 tests. Then use disposable files in both applications:

1. Save a supported scene in ScriptCraft and inspect SceneProperties indentation against the Final Draft reference convention.
2. Reopen and Save repeatedly; formatting must not accumulate whitespace or change matched nested IDs.
3. Verify JOHN/Arc 1 and unrelated roster edits in ScriptCraft and Final Draft.
4. Save a separate copy from Final Draft and observe its nested UUID regeneration without treating that difference alone as a defect.
5. Confirm deferred/unsupported arc content and non-SceneProperties opaque source remain unchanged.
6. Confirm Save, Save As, reopen, and ordinary screenplay editing still succeed after V5-WO-002.

Expected: readable reference-style SceneProperties XML, stable retained IDs and arc content, no formatting-driven code 30, and no requirement to mimic Final Draft's nested UUID churn.

## Handoff

After implementation records are complete, a separately commissioned final agent may commit and merge the completed work into master under the user's explicit instruction. That agent must not run validation or claim it passed; all checks remain user-owned.

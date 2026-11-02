# FDX Fixes v5 — diagnostic code 30 normalized source-tree mismatch

**Status:** V5-WO-001–003 implementations and requested regressions are authored; all tests remain unexecuted and user-owned validation is pending. No builds, tests, formatting, lint, manual verification, desktop checks, implementation checks, reviews, independent validators, or Final Draft interoperability checks were run; repository policy leaves all validation to the user.

## ELI5 — what is failing?

Think of Save as making a draft, printing it, and immediately reading that printed copy back before replacing the original. ScriptCraft prepares an expected copy of the document in the session, turns it into new FDX bytes, then deserializes those new bytes and compares the result with the expected copy.

If those copies disagree, the failsafe stops before touching the original file. That prevents us from overwriting good work with a file that does not faithfully represent the session. Code 30 means the ordinary screenplay text and the set of IDs agree, but something else in the preserved XML structure or its metadata disagrees. In this case, the new diagnostics identified a duplicate newline between the XML declaration and the root element.

## Confirmed code 30 mechanism — duplicate declaration-adjacent newline

Two user diagnostics identify the same top-level whitespace mismatch:

> 30: ... path=section[0]/node[0]; field=text.content; expected=length=1, whitespace-only; actual=length=2, whitespace-only

> 30: ... path=section[0]/node[0]; field=child.order; expected=text:length=1, whitespace-only; actual=text.length=2, whitespace-only

The different field labels do not indicate different root causes. Both identify the first preserved document node and a one-character increase in whitespace.

The source mechanism is deterministic:

1. Final Draft references begin with an XML declaration followed by one LF before FinalDraft, as shown in [`reference2.fdx`](../../.reference/reference2.fdx:1).
2. The reader validates but does not retain the declaration event at [`fdx.rs`](../../crates/scriptcraft-core/src/fdx.rs:2311). It does retain the ordinary top-level LF text node between the declaration and root.
3. The session's expected tree therefore starts with text length 1.
4. [`XML_DECLARATION`](../../crates/scriptcraft-core/src/fdx.rs:41) already ends with LF. [`write_v6_inner()`](../../crates/scriptcraft-core/src/fdx.rs:582) writes that declaration and then writes the retained LF text node again.
5. The output has two LFs between declaration and root. Re-reading produces text length 2 while the expected tree still has length 1, triggering code 30.

This explains why valid existing documents fail even when canonical screenplay content, IDs, SceneProperties, and opaque source data are otherwise unchanged. New preservation-free documents do not take the source-tree comparison branch, which is consistent with first saves succeeding and reopened saves failing.

The repair must stop the extra newline from being emitted. Merely teaching the comparison to ignore length 1 versus length 2 would allow whitespace to accumulate on every save and would not fix the underlying writer defect.

## Executive finding

The user now reports Save failing with diagnostic code 30. The centralized map in [`technical-context.md`](../../.forge/blueprints/technical-context.md:80) assigns code 30 to the strict normalized expected-source-tree versus re-read-source-tree comparison in [`write_v6_inner()`](../../crates/scriptcraft-core/src/fdx.rs:514).

This is a substantial localization:

- the candidate passed project and preservation validation;
- the original source graph matched;
- managed source reconciliation completed;
- the serialized output could be read again;
- semantic title/body elements matched;
- registered identity sets matched;
- the full normalized preservation trees still differed.

The failing check is at [`fdx.rs`](../../crates/scriptcraft-core/src/fdx.rs:617), and the error is returned at [`fdx.rs`](../../crates/scriptcraft-core/src/fdx.rs:624).

Code 30 therefore does not mean that canonical screenplay text or the registered identity set is already known to be lost. It means that some other preserved source structure or metadata differs between the detached candidate and the tree reconstructed from the emitted XML. The two user reports identify that difference as one added top-level LF.

## V4 summary and carried-forward state

### V4-WO-001 — Empty managed paragraphs

Implementation authored the native no-Text-child shape for empty managed body/title-role Paragraphs, synchronized semantic/source runs through clear and retype operations, and omitted serializer-invisible empty XML text nodes during expected/actual normalization. The principal changes are in [`fdx.rs`](../../crates/scriptcraft-core/src/fdx.rs), [`preservation.rs`](../../crates/scriptcraft-core/src/preservation.rs), [`editing.rs`](../../crates/scriptcraft-core/src/editing.rs), and [`cast_list.rs`](../../crates/scriptcraft-core/src/cast_list.rs). Authored coverage is in [`v4_wo001_empty_managed_paragraphs.rs`](../../crates/scriptcraft-core/tests/v4_wo001_empty_managed_paragraphs.rs).

The user confirms this did not resolve the existing-document Save failure.

### V4-WO-002 — Generated SceneProperties and retained Character arcs

Implementation authored generated SceneProperties/SceneArcBeats containers for supported managed headings while retaining supported CharacterArcBeat subtrees by normalized Character association. Supported non-arc fields regenerate from shared defaults; ambiguous arc scenes retain their original complete subtree through per-scene deferral. The principal implementation is in [`scene_metadata.rs`](../../crates/scriptcraft-core/src/scene_metadata.rs), with authored coverage in [`v4_wo002_generated_scene_properties.rs`](../../crates/scriptcraft-core/tests/v4_wo002_generated_scene_properties.rs).

The user confirms this also did not resolve the Save failure.

### V4-WO-003 — Numeric diagnostics

Implementation added stable Save/Save As diagnostic integers and private writer-stage/structural-difference support. The public variant [`FdxError::SaveDiagnosticUnsupportedConstruct`](../../crates/scriptcraft-core/src/fdx.rs:65) preserves an integer prefix through storage and desktop error details. The complete 1–34 map is recorded in [`technical-context.md`](../../.forge/blueprints/technical-context.md:45), with authored transaction coverage in [`v4_wo003_residual_save_localization.rs`](../../crates/scriptcraft-core/tests/v4_wo003_residual_save_localization.rs).

Code 30 is the first user-supplied runtime result that identifies the residual gate. V4's full execution records remain in [`fdx-fixes-v4.md`](fdx-fixes-v4.md:253). All regression coverage remains user-unvalidated unless the user separately records a result.

## 1. What code 30 compares

For source-backed documents, [`write_v6_inner()`](../../crates/scriptcraft-core/src/fdx.rs:514) currently:

1. clones the candidate preservation sections;
2. sets the native root Version;
3. reconciles managed semantic elements into that clone;
4. applies best-effort inserted-paragraph sibling formatting;
5. retains the result as the expected source tree;
6. serializes the same nodes;
7. re-reads the output into an actual source tree;
8. compares semantic elements and identity sets;
9. compares normalized expected and actual trees.

[`normalize_sections()`](../../crates/scriptcraft-core/src/fdx.rs:757) recursively applies [`normalize_source_tree()`](../../crates/scriptcraft-core/src/fdx.rs:946) and [`normalize_text_nodes()`](../../crates/scriptcraft-core/src/fdx.rs:964).

Current normalization deliberately:

- removes empty XML text nodes because the writer emits no bytes for them;
- merges adjacent XML text nodes because parser event segmentation is not a semantic source difference;
- clears Managed ownership and Managed anchors because internal semantic/anchor IDs are reallocated during re-read.

The remaining equality comparison is broader than XML tag names and text. It includes section metadata, ordered children, element names, ordered attributes and values, namespace bindings, text/CDATA/comments/processing instructions, and preservation metadata not cleared by normalization.

## 2. SceneProperties IDs and Final Draft's regeneration behavior

Yes. SceneProperties-related IDs participate in both identity validation and the code 30 source-tree comparison, but at different levels:

| Check | SceneProperties-related meaning |
|---|---|
| Original non-Paragraph identity graph | [`identity_domain()`](../../crates/scriptcraft-core/src/fdx.rs:3013) classifies SceneProperties IDs in the Scene domain and SceneArcBeat/Beat IDs in the Beat domain. [`same_non_paragraph_identities()`](../../crates/scriptcraft-core/src/fdx.rs:638) requires those registered identities to remain consistent with the original graph. A change here fails earlier than code 30. |
| Code 29 identity-set comparison | [`identity_set()`](../../crates/scriptcraft-core/src/fdx.rs:646) includes registered nested arc Paragraph IDs along with other identities. Adding or losing a registered ID should fail at code 29 before the tree comparison. |
| Code 30 ordered source-tree comparison | Raw ID attributes, their values, and their placement on specific elements remain part of the ordered XML tree. Equal identity sets do not prove that IDs stayed attached to the same elements, so code 30 can still catch a moved ID or another attribute mismatch. |

The current reconciler does not imitate Final Draft's regenerate-every-time behavior. A matched Character retains its existing nested arc Paragraph and ID at [`scene_metadata.rs`](../../crates/scriptcraft-core/src/scene_metadata.rs:322); only a newly introduced normalized Character receives a fresh allocation at [`scene_metadata.rs`](../../crates/scriptcraft-core/src/scene_metadata.rs:326), and removed derived identities are unregistered at [`scene_metadata.rs`](../../crates/scriptcraft-core/src/scene_metadata.rs:370).

Therefore, Final Draft producing fresh nested UUIDs on every Save does **not by itself explain code 30**. ScriptCraft compares its own candidate with its own immediate re-read; Final Draft has not rewritten anything between those two operations. The observation does reinforce the existing rule that nested UUIDs are not durable Character-association keys. It also makes an ID-placement/attribute difference a relevant code 30 diagnostic case, while a simple changed Scene/Beat identity or added/lost nested Paragraph identity should already have failed at code 4 or code 29 respectively.

## 3. Confirmed diagnostic limitation

[`first_source_tree_difference()`](../../crates/scriptcraft-core/src/fdx.rs:835) already computes a private first difference after the code 30 mismatch. It currently provides a child-index path, node kind, expanded element name, and child/attribute counts to crate-internal tests.

That is not yet sufficient for a user-driven fix:

- ordinary application errors render only integer 30 and the generic construct message;
- the private path is not available in the reported desktop detail;
- the helper does not explicitly classify every field participating in full equality;
- differences in source Type metadata, non-Managed ownership, dependency confidence, or non-Managed anchors are not directly categorized by the helper;
- an attribute mismatch is not distinguished from a namespace-binding mismatch beyond the containing node summary;
- text mismatches do not report even value-free classifications such as text length or whitespace-only status.

The helper must be made exhaustive enough that every actual full-tree inequality produces a concrete difference category. Otherwise it can remain unhelpful even after its metadata is surfaced.

### Diagnostic category glossary and actual reference evidence

The category list describes everything the current in-memory comparison can represent. It is not a claim that every category occurs in Final Draft files. A scan of all four checked-in Final Draft references—[`reference.fdx`](../../.reference/reference.fdx:1), [`reference2.fdx`](../../.reference/reference2.fdx:1), [`arc copy.fdx`](../../.reference/arc%20copy.fdx:1), and [`real_script_example.fdx`](../../.reference/real_script_example.fdx:1)—found no CDATA, XML comments, non-declaration processing instructions, namespace declarations, or xml:space attributes.

| Category | What it means in the comparison | Evidence in the Final Draft references | Evidence in ScriptCraft-generated FDX |
|---|---|---|---|
| Section metadata | The internal preservation wrapper's name, kind, and ordinal. This is bookkeeping around the parsed document, not an FDX XML element. | Not present in the files; it is created by ScriptCraft's reader. | Same internal reader bookkeeping. It cannot be inspected as an FDX field. |
| Element expanded name | An element's local name plus namespace URI when one exists—for example, Paragraph or SceneProperties. | Element local names are pervasive. No namespace declarations were found, so observed names are unnamespaced. | Generated local names such as FinalDraft, Content, Paragraph, Text, ElementSettings, SceneProperties, and CharacterArcBeat; normally no namespace URI. |
| Attributes | Ordered XML attributes and their values, including Type, id, Length, Page, Style, Alignment, and many others. | Extensively observed throughout every reference. Nested arc Paragraph IDs occur, although no SceneProperties or CharacterArcBeat element's own ID was found in the reference scan. | Generated Type, id, SceneProperties defaults, ElementSettings data, and nested arc Paragraph attributes. Attribute differences are a leading code 30 candidate category. |
| Namespace bindings | Explicit xmlns or xmlns:prefix declarations that bind XML prefixes/default namespaces. | None found in the four references. | Not emitted by ordinary new-document generation. The reader can preserve them from synthetic or other imported XML. |
| Source Type metadata | The internal source_type field copied from an element's Type attribute. It is related to, but separate from, the raw attribute comparison. | Type occurs on managed Paragraphs, DynamicLabel elements, ElementSettings, ParagraphSpec, and other elements. | Generated managed Paragraph and ElementSettings types; the reader reconstructs source_type on reopen. |
| Ownership | Internal Managed, Opaque, or Attached classification indicating whether ScriptCraft edits a node, preserves it, or associates it with another identity. It is not an XML attribute. | Not present in the files. The reader derives Managed or Opaque from position and Type. Attached occurrences currently found are test fixtures, not observed Final Draft source. | Internal reader/session state only. Managed ownership and anchors are normalized before comparison; other ownership differences still matter diagnostically. |
| Dependency confidence | Internal metadata describing confidence in mapped relationships. It is not serialized into FDX. | Not present in the files. | Internal state, normally reconstructed as Unknown by the FDX reader. It is a comparison-completeness case, not a leading reference-backed hypothesis. |
| Anchors | Internal session/source anchor IDs used to associate managed semantic elements with preserved nodes. These are not the UUIDs stored in FDX. | Not present in the files. FDX id attributes are separate. | Created internally during read/edit operations. Managed anchors are deliberately ignored during normalized comparison because re-read allocates new internal values. |
| Node kind | Whether a preserved child is an Element, text, CDATA, comment, or processing instruction. | Elements and ordinary text nodes are observed throughout. The other kinds were not found. | Ordinary generation emits Elements and text. The XML declaration is emitted separately and is not evidence of a preserved processing-instruction node. |
| Child count/order | The number and exact order of children beneath an element or section, including Paragraphs, Text runs, SceneProperties, and structural whitespace. | Pervasive and directly visible in every reference. Final Draft's element order and indentation produce many ordered text/element children. | Generated Content, ElementSettings, SceneProperties, arc beats, and structural whitespace all have ordered children. This is a leading code 30 category. |
| Text differences | Ordinary XML character data, including screenplay content and indentation/newline whitespace. Adjacent parser text events are merged for comparison. | Pervasive in Text elements and as formatting whitespace between elements. | Generated screenplay text and deterministic structural whitespace. Empty text nodes are already normalized because they emit no bytes. |
| CDATA differences | XML CDATA sections such as a literal character-data block. This is distinct from an ordinary Text element. | None found in the four references. | Not generated by ordinary ScriptCraft documents. It appears only in parser/preservation support and synthetic tests currently, so it is not a leading Final Draft hypothesis. |
| Comment differences | XML comments, not screenplay notes or Final Draft ScriptNotes. | None found in the four references. ScriptNotes elements are ordinary elements and do not establish XML-comment usage. | Not generated ordinarily; supported only for imported/synthetic preservation. |
| Processing-instruction differences | XML processing instructions other than ordinary elements/text. The standard XML declaration is handled separately and is not retained as a preserved PI child. | Only the standard XML declaration was found; no non-declaration PI was found. | The declaration is generated, but no preserved PI child is generated ordinarily. Synthetic PI preservation tests are not Final Draft evidence. |
| Text length/whitespace classification | Derived diagnostic metadata about an ordinary text node: its length and whether it is empty or whitespace-only. It lets diagnostics distinguish formatting whitespace without revealing screenplay content. | Derived from the observed text and indentation, not a separate FDX construct. | Derived from generated/imported text. This is safe diagnostic detail, not another kind of FDX element. |

The practical priority is therefore attributes and ID placement, element names, ordered children, ordinary text/whitespace, and the internal metadata that participates in equality. CDATA, comments, non-declaration PIs, and namespace declarations remain valid generic preservation controls, but there is no evidence in these Final Draft files to make them leading code 30 suspects. Synthetic coverage proves only that ScriptCraft supports those XML forms, not that Final Draft produces them in the user's documents.

## 4. Source areas investigated and current candidate ranking

### A. General serializer/parser fixed-point mismatch — highest-priority control

A parsed source tree should serialize and re-read to the same normalized tree when no candidate mutation occurs. A minimal Action-only fixture can test this without SceneProperties, Cast Lists, arcs, or empty managed paragraphs.

If that minimal unchanged fixture produces code 30, the leading cause is a general codec asymmetry rather than V4-WO-002 scene reconciliation. Candidate fields include XML text/CDATA segmentation, processing instructions, namespace data, attributes, or nonserialized preservation metadata. No specific field is confirmed yet.

### B. Derived SceneProperties candidate mismatch — high-priority comparative control

Every Save runs [`EditingSession::prepare_cast_list_save_candidate()`](../../crates/scriptcraft-core/src/editing.rs:205). V4-WO-002 regeneration in [`reconcile_heading()`](../../crates/scriptcraft-core/src/scene_metadata.rs:206) replaces supported SceneProperties containers and reconciles retained arc children before writer comparison.

A direct write of the freshly read Project must therefore be compared with a write of the corrected save candidate:

- direct unchanged write failing indicates a general source fixed-point problem;
- direct unchanged write passing while corrected-candidate writing fails indicates a derived-candidate problem;
- failure only after a Character roster edit narrows the candidate further to arc membership/order/identity-associated source structure;
- failure with no Scene Heading argues against SceneProperties as the universal cause.

The current source does not justify regenerating or removing more SceneProperties data merely because this remains a candidate.

### C. Managed empty/nonempty projection mismatch — focused control

V4-WO-001 changed managed source and semantic run shapes. Code 30 occurs after semantic comparison, so a simple semantic text/run mismatch is not the reported gate. However, managed source Text removal, residual empty CDATA, source attributes, or attached non-Text children can still produce a tree-only mismatch.

Blank Paragraph, clear/retype, multi-run, and text-plus-CDATA controls should remain in the matrix. They are diagnostic controls, not evidence that V4-WO-001 caused the failure.

### D. Nonserialized preservation metadata mismatch — comparison completeness case

Full element equality includes metadata beyond serialized XML. Normalization clears only Managed ownership/anchors. Other metadata differences can therefore theoretically cause code 30 even when emitted XML is structurally equivalent.

Attached ownership was investigated. Current Attached construction found in [`editing.rs`](../../crates/scriptcraft-core/src/editing.rs:1383) and [`preservation.rs`](../../crates/scriptcraft-core/src/preservation.rs:1924) is test-fixture code rather than a leading production import path. The production FDX preservation path constructs Managed or Opaque nodes. Attached ownership is therefore not the primary runtime hypothesis, but the diff helper should still classify it if it ever participates in a mismatch.

### E. Identity placement or attribute mismatch despite equal identity sets — retained possibility

The code 29 identity-set check proves set equality, not that every ID remains attached to the same source occurrence. A swapped or misplaced existing ID can produce code 30 while preserving the set. Attribute/order comparison remains necessary; code 30 must not be downgraded to a semantic-only or set-only check.

## 5. Required minimal evidence matrix

For each fixture, distinguish three operations:

1. Freshly read Project → direct [`write_v6()`](../../crates/scriptcraft-core/src/fdx.rs:490).
2. Freshly read Project → detached derived save candidate → direct writer.
3. Full filesystem Save and Save As through [`save_session()`](../../crates/scriptcraft-core/src/fdx_storage.rs:344).

Use this fixture order:

1. Minimal Action-only source with no Scene Heading, Cast List, arc data, or opaque payload.
2. Minimal blank Action with no Text child.
3. Scene Heading with empty SceneArcBeats.
4. Supported scene with JOHN/Arc 1 and no other Characters.
5. Disposable copy of [`arc copy.fdx`](../../.reference/arc%20copy.fdx:1).
6. Disposable copy of [`reference2.fdx`](../../.reference/reference2.fdx:1).
7. The user's actual failing document reduced only as necessary for privacy and size.

Record the first code 30 structural path and field category before attempting a repair. If a fixture passes direct writing but fails full Save, compare its derived candidate rather than changing general serializer behavior.

## 6. Work-order candidates

V5-WO-001 is approved and implemented as a diagnostic-only order. Later repair branches remain conditional on user reproduction evidence.

### Candidate V5-WO-001 — Exhaustive code 30 structural diagnostics

**Status:** Implemented; validation and runtime evidence pending.

Extend the existing private difference machinery so every full normalized-tree inequality yields one exhaustive, privacy-safe category and ordered node path. Categories should cover section metadata, element expanded name, attributes, namespace bindings, source Type metadata, ownership, dependency confidence, anchors, node kind, child count/order, text/CDATA/comment/PI differences, and text length/whitespace classification where useful.

Surface a bounded form of that category and path alongside the existing code 30 prefix in the desktop detail. Do not expose screenplay text, arc text, attribute values, identity values, XML, encoded images, or filesystem paths. Preserve code 30 and all strict failure behavior.

Add unit fixtures proving that each full-equality-only field produces a non-empty correct difference classification. Add desktop formatting coverage proving the path/category survives the transactional error path.

### Candidate V5-WO-002 — Declaration-adjacent whitespace fixed-point repair

**Status:** Implemented; regression coverage authored; validation pending.

Separate declaration spelling from declaration-to-root separation for source-backed output. The recommended bounded fix is to emit the declaration without adding its own trailing LF for preserved documents, then emit the retained top-level nodes unchanged. Existing top-level whitespace—or its absence—therefore round-trips exactly instead of receiving one additional LF per save. Keep the current canonical declaration-plus-LF behavior for newly generated preservation-free documents.

Add focused fixtures for one LF, multiple LFs, spaces/tabs, no whitespace, no original declaration, and top-level comments/PIs as generic controls. Prove exactly one declaration, unchanged retained top-level whitespace, direct fixed-point serialization, repeated Save/Save As, and no code 30 on the Final Draft references. Do not normalize away whitespace inside the root or weaken the strict tree comparison.

### Candidate V5-WO-003A — General codec fixed-point repair

**Condition:** A minimal unchanged source-backed fixture fails direct serialization, especially outside SceneProperties.

Repair the exact parser/writer/normalization asymmetry identified by V5-WO-001 and V5-WO-002. If the differing field is nonserialized runtime metadata, define its comparison ownership explicitly without ignoring actual XML differences. If the field is serialized XML structure, fix generation/parsing rather than excluding it from comparison.

Retain strict semantic, identity-placement, attribute, namespace, opaque-content, and transaction checks. This candidate must be rewritten with the concrete field and fixture before implementation.

### Candidate V5-WO-003B — Derived SceneProperties candidate repair

**Condition:** Direct unchanged serialization passes but derived-candidate serialization fails at a SceneProperties/SceneArcBeats path.

Repair the specific generated-container or retained-arc mutation causing the mismatch. Preserve V4-WO-002's normalized-name arc association, supported arc content, generated non-arc policy, and per-scene deferral. Do not respond by skipping derived correction, deleting ambiguous arc data, or bypassing source-tree comparison for SceneProperties.

This is an alternative targeted branch to V5-WO-003A, not an automatically sequential additional rewrite.

### Candidate V5-WO-003C — Managed text/source projection repair

**Condition:** The first difference is isolated to a managed Paragraph Text/run/attachment path, including blank or clear/retype fixtures.

Repair the exact managed source projection while retaining V4-WO-001's native empty Paragraph shape and semantic/source synchronization. Do not globally normalize away CDATA, opaque children, attributes, or source-owned attachments.

This is another conditional branch, selected only if the structural evidence places the defect here.

## V5-WO-001 execution log — 2026-11-02

### Implementation outcome

The strict normalized-tree comparison still rejects every mismatch with code 30. After that comparison fails, the first-difference walker now classifies every equality field in preservation sections, elements, attributes, namespace bindings, metadata, ordered child nodes, and text-like node kinds. Traversal is deterministic: section order, element fields in declared comparison priority, then child indexes in document order. Public detail has the form `path=section[N]/node[N]...; field=CATEGORY; expected=SUMMARY; actual=SUMMARY`, is capped at 240 bytes and 12 child indexes, and does not include raw text/content, attribute/namespace values, IDs/UUIDs/anchors, serialized XML, image data, or paths. Code 30 remains the same failure gate; no serializer, normalization, SceneProperties, identity, or transaction behavior was changed by V5-WO-001. The subsequent user diagnostics localized the production defect to duplicate declaration-adjacent LF output; that defect remains unfixed.

### Changed files

- [`crates/scriptcraft-core/src/fdx.rs`](../../crates/scriptcraft-core/src/fdx.rs): exhaustive structural categories, deterministic value-free first-difference detail, and dedicated public code 30 error variant.
- [`crates/scriptcraft-core/src/fdx_storage.rs`](../../crates/scriptcraft-core/src/fdx_storage.rs): allow the owned code 30 detail to pass through storage error types and retain it in transactional display.
- [`apps/scriptcraft/src/main.rs`](../../apps/scriptcraft/src/main.rs): desktop detail projection regression coverage; the existing formatter passes the full transactional message unchanged.
- [`crates/scriptcraft-core/tests/v5_wo001_code30_structural_diagnostics.rs`](../../crates/scriptcraft-core/tests/v5_wo001_code30_structural_diagnostics.rs): public codec, Save, Save As, privacy, and no-publication regression authored.
- [`crates/scriptcraft-core/src/fdx.rs`](../../crates/scriptcraft-core/src/fdx.rs): internal synthetic field-category and equal-tree control tests authored.
- [`.forge/blueprints/technical-context.md`](../../.forge/blueprints/technical-context.md) and [`.forge/requirements/features/screenplay-writing-loop.md`](../../.forge/requirements/features/screenplay-writing-loop.md): durable diagnostic contract/schema updated.
- [`work-orders-v5/WO-001-exhaustive-code30-diagnostics.md`](work-orders-v5/WO-001-exhaustive-code30-diagnostics.md): status and implementation/evidence handoff updated.

### Authored but not executed

- Internal normalized source-tree tests cover section local name/kind/ordinal; element local/namespace/qualified-name; source Type, ownership, dependency confidence, anchors; attribute name/order/count/value; namespace-binding prefix/order/count/namespace; ordinary text, CDATA, comment, PI target/data, generic node-kind; child presence/order; empty-versus-whitespace text after normalization; privacy; and equal-tree no-difference behavior. Existing structural-name coverage asserts the concrete path/category.
- [`v5_wo001_code30_structural_diagnostics.rs`](../../crates/scriptcraft-core/tests/v5_wo001_code30_structural_diagnostics.rs) authors a reference-backed internal source-Type metadata mismatch through direct codec, transactional Save, and Save As paths, asserting code 30/category retention, private marker absence, unchanged session/target, unchanged source bytes, and no Save As destination publication. It also authors a checked-reference ID-value swap that preserves the identity set while requiring the code 30 attribute category and withholding the ID values.
- Desktop unit coverage asserts the path/category survives `show_fdx_save_error()` and document-unchanged status remains.
- Repository policy forbids agent-run tests, builds, formatting, lint, manual verification, implementation checks, reviews, and independent validators. None were run.

### User evidence received

Two independent documents produced code 30 at section 0/node 0 with expected top-level whitespace length 1 and actual length 2. Source inspection confirmed the duplicate declaration-adjacent LF mechanism recorded at the top of this document. This is sufficient to propose the concrete V5-WO-002 writer repair. User-owned validation remains necessary after implementation to establish that ordinary Save/Save As succeeds and no independent mismatch remains.

## V5-WO-002 execution log — 2026-11-02

### Implementation outcome

At writer initialization, preservation-backed projects now emit the standard declaration without its own trailing LF, then serialize every retained top-level node through the existing node writer without trimming or normalization. Preservation-free generation still uses the original declaration-plus-one-LF constant. The strict normalized source-tree comparison and code 30 failure path remain unchanged. No SceneProperties, identity, managed text, storage, conflict, or transaction behavior was changed.

### Changed files

- [`crates/scriptcraft-core/src/fdx.rs`](../../crates/scriptcraft-core/src/fdx.rs): added the declaration-without-LF spelling and selected it only when preservation is present.
- [`crates/scriptcraft-core/tests/v5_wo002_declaration_whitespace_fixed_point.rs`](../../crates/scriptcraft-core/tests/v5_wo002_declaration_whitespace_fixed_point.rs): authored direct-codec whitespace variants, fixed-point/repeated serialization, preservation-free output, code 30 negative control, filesystem reference Save/Save As/reopen cycles for both references, and an external-conflict transaction control.
- [`.forge/requirements/features/screenplay-writing-loop.md`](../../.forge/requirements/features/screenplay-writing-loop.md) and [`.forge/blueprints/technical-context.md`](../../.forge/blueprints/technical-context.md): recorded generated versus source-backed separator ownership.
- [`work-orders-v5/WO-002-declaration-whitespace-fixed-point.md`](work-orders-v5/WO-002-declaration-whitespace-fixed-point.md): status and execution handoff updated.
- [`fdx-fixes-v5.md`](fdx-fixes-v5.md): this implementation record and validation boundary.

### Authored but not executed

- Direct codec cases cover one LF, multiple LFs, spaces/tabs, no separator, missing declaration with and without retained whitespace, and top-level comment/PI order. Each checks the exact declaration-adjacent output prefix, one declaration, and a second serialization after reopen.
- Preservation-free generation asserts the existing declaration-plus-one-LF convention. An independently injected preserved source-Type metadata mismatch still asserts the dedicated code 30 error.
- Filesystem coverage performs unchanged Save, repeated Save after reopen, unchanged Save As with source-byte retention, and a second save after reopening the Save As output for both [`reference2.fdx`](../../.reference/reference2.fdx:1) and [`arc copy.fdx`](../../.reference/arc%20copy.fdx:1). An external-modification conflict control asserts the destination, live session, and save target remain unchanged.
- Repository policy prohibits agent-run tests, builds, formatting, lint, manual verification, implementation checks, reviews, and independent validators. None were run; authored assertions do not constitute passing validation.

### Remaining user-owned validation

Run the applicable formatting, lint, build, workspace, and focused V5-WO-002 tests, then perform the disposable-copy ScriptCraft/Final Draft Save, Save As, reopen, and repeated-save sequence in the work order. Record any remaining code 30 with its complete path/category. Until that is done, this is an implementation handoff, not a claim that the fix passed runtime or interoperability validation.

## V5-WO-003 execution log — 2026-11-02

### Implementation outcome

After an individual managed scene candidate has passed the existing supported-scene reconciliation boundary, only its generated SceneProperties/SceneArcBeats/CharacterArcBeat wrapper structure receives deterministic LF/two-space indentation. The formatter computes each line's depth from the managed heading's actual location in the preserved tree, preserves retained nested Paragraph subtrees exactly, leaves empty SceneArcBeats childless, and skips contexts with effective xml:space preservation or mixed/opaque structure. Deferred scenes are not formatted. Formatting is applied to the candidate preservation tree before the writer clones its expected sections; code 30 comparison remains strict and unchanged. No Final Draft nested-ID regeneration was implemented.

### Changed files

- [`crates/scriptcraft-core/src/scene_metadata.rs`](../../crates/scriptcraft-core/src/scene_metadata.rs): bounded, actual-depth formatting after successful supported-scene reconciliation, with xml:space and element-only safeguards.
- [`crates/scriptcraft-core/tests/v5_wo003_scene_properties_formatting.rs`](../../crates/scriptcraft-core/tests/v5_wo003_scene_properties_formatting.rs): authored direct-codec formatting, empty-arc, multiple-Character, retained arc, deferred-scene, xml:space, code 30 negative-control, and filesystem Save/Save As/reopen/repeated-save regressions.
- [`.forge/requirements/features/screenplay-writing-loop.md`](../../.forge/requirements/features/screenplay-writing-loop.md) and [`.forge/blueprints/technical-context.md`](../../.forge/blueprints/technical-context.md): recorded bounded formatting and the compatibility decision.
- [`work-orders-v5/WO-003-scene-properties-formatting.md`](work-orders-v5/WO-003-scene-properties-formatting.md): status updated to implementation authored with user-owned validation pending.
- [`fdx-fixes-v5.md`](fdx-fixes-v5.md): this implementation and test-coverage record.

### Authored but not executed

The new regression file covers correctly indented generated containers before the heading Text child, actual depth for nested Character arcs, empty self-closing SceneArcBeats, unchanged retained JOHN arc subtree/ID, deterministic direct serialization, filesystem Save and Save As/reopen/repeated Save, deferred unsupported-scene preservation, xml:space formatting exclusion, and an independently injected code 30 source-tree mismatch. These are authored assertions only, not passing validation.

### Deferred compatibility TODO

[`technical-context.md`](../../.forge/blueprints/technical-context.md) records the user-observed Final Draft behavior of regenerating nested scene-arc Paragraph UUIDs on every save. ScriptCraft retains matched IDs and allocates only for new derived entries. Revisit only with concrete interoperability evidence that regeneration is required for semantics, references, or reopen behavior; no UUID regeneration work is included here.

### Validation boundary

Repository policy prohibited agent-run tests, builds, formatting, lint, manual verification, implementation checks, reviews, and independent validators. None were run. The user owns all validation, including the reference Save/Save As/reopen/repeated-save matrix and any desktop interoperability observation. No commit or merge was performed.

## Durable documents after a fix is selected

- [`technical-context.md`](../../.forge/blueprints/technical-context.md:45): extend the code 30 diagnostic contract and record the final comparison/serialization ownership rule.
- [`screenplay-writing-loop.md`](../../.forge/requirements/features/screenplay-writing-loop.md:154): amend only the specific round-trip behavior corrected by the selected branch.
- [`product-overview.md`](../../.forge/requirements/product-overview.md:20): update only if the final repair changes a user-visible preservation guarantee.
- [`fdx-fixes-v4.md`](fdx-fixes-v4.md:271): retain as historical implementation evidence; v5 is the active code 30 strategy record.

## Validation boundary and closeout criteria

No agent-run checks are permitted. The user owns all test execution, desktop reproduction, and Final Draft checks.

V5 diagnosis is complete only when a user-facing code 30 detail identifies a bounded path/category and a minimal fixture reproduces it. V5 repair is complete only when that fixture and the existing Save/Save As/reopen matrix pass under user validation, repeated saves remain fixed-point stable, retained arc content and opaque source survive, and deliberate semantic/identity/tree corruption still fails transactionally.

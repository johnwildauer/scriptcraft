# V5-WO-001 — Exhaustive code 30 structural diagnostics

- **Status:** Implemented; user-owned validation and reproduction pending
- **Depends on:** V4-WO-003 numeric diagnostics and the code 30 investigation in [`fdx-fixes-v5.md`](../fdx-fixes-v5.md)
- **Validation:** User-owned; no agent-run checks or independent validators

## Objective

Make diagnostic code 30 identify the exact normalized source-tree difference that blocks Save. Preserve the existing integer 30 prefix and strict failsafe, but append a bounded, privacy-safe node path and field/category description that lets the user report which candidate attribute, XML structure, or internal preservation field differs.

This is a diagnostic order only. Do not change serialization, normalization, scene reconciliation, identity handling, or save transaction behavior.

## Current evidence and limitation

The user confirms Save fails with code 30. [`write_v6_inner()`](../../../crates/scriptcraft-core/src/fdx.rs:514) reaches this error only after output re-read, semantic comparison, and identity-set comparison succeed. The normalized expected and actual trees differ at [`fdx.rs`](../../../crates/scriptcraft-core/src/fdx.rs:617).

[`first_source_tree_difference()`](../../../crates/scriptcraft-core/src/fdx.rs:835) computes only crate-private metadata and does not explicitly classify every field included in full equality. Ordinary desktop errors therefore do not reveal whether the mismatch concerns attributes, ID placement, child order, ordinary text, or internal metadata.

The reference-evidence glossary in [`fdx-fixes-v5.md`](../fdx-fixes-v5.md) establishes that attributes, IDs, element names, child order, ordinary text, and whitespace are observed in Final Draft files. CDATA, comments, non-declaration processing instructions, and namespace declarations were not found in the four checked-in references and are not leading hypotheses.

## Scope

1. Extend source-tree difference detection to cover every field that participates in normalized full-tree equality.
2. Return one deterministic first-difference node path and specific field/category for every code 30 mismatch.
3. Expose a bounded form in the public codec error and desktop detail while retaining the code 30 prefix at character zero of the underlying diagnostic.
4. Prioritize reference-backed categories in the public wording and regression coverage.
5. Add exhaustive synthetic unit coverage for internal-only and generic XML fields so the helper cannot silently return no useful difference.
6. Update durable diagnostic documentation and record the implementation without claiming the residual production defect is fixed.

## Non-goals

- No production fix for code 30 and no change to which documents fail or succeed.
- No relaxation of semantic, identity, source-tree, conflict, or atomic-save checks.
- No SceneProperties, UUID, managed-empty-paragraph, or Cast List behavior changes.
- No screenplay text, arc text, attribute values, identity values, XML payloads, image payloads, or filesystem paths in diagnostics.
- No claim that CDATA, comments, processing instructions, or namespaces occur in the user's Final Draft documents.
- No agent-run tests, desktop automation, Final Draft automation, review, or validation.

## Implementation requirements

### 1. Make first-difference detection exhaustive

Refactor [`first_source_tree_difference()`](../../../crates/scriptcraft-core/src/fdx.rs:835) and its node-sequence helper so their comparison inputs and detected fields correspond to the actual normalized equality comparison.

The difference representation shall classify, as applicable:

- preservation section name, kind, and ordinal;
- element expanded/local name;
- ordered attributes, with attribute local name where safely useful but never its value;
- namespace bindings, reporting binding prefix/presence rather than values when possible;
- source Type metadata;
- ownership category without StableId/foreign identity values;
- dependency-confidence category;
- anchor presence/category without anchor values;
- node kind;
- child count, order, missing child, or unexpected child;
- ordinary text, CDATA, comment, and processing-instruction differences;
- ordinary-text length and empty/whitespace-only classification without content.

The existing normalization in [`normalize_source_tree()`](../../../crates/scriptcraft-core/src/fdx.rs:946) already clears Managed ownership and anchors. Do not broaden that normalization. Differences in fields it intentionally retains must receive a category rather than being ignored by the diagnostic helper.

For every deliberately constructed inequality in the full normalized structures, the helper shall return a concrete difference. If full equality says two normalized trees differ, the diagnostic helper must not return an empty or generic unknown result merely because the field was not previously handled.

### 2. Keep diagnostics deterministic and privacy-safe

Report the first difference in document order using an ordered child-index path prefixed by the preservation-section index. Define deterministic tie-breaking for metadata versus child differences so repeated runs produce the same detail.

Public detail may include:

- node path;
- element local name and namespace-presence classification;
- difference field/category;
- expected/actual node kind;
- expected/actual child or attribute counts;
- differing attribute local name, if established;
- expected/actual ownership/dependency categories, if established;
- text length and empty/whitespace-only classification.

Public detail shall not include raw text, CDATA/comment/PI content, attribute values, ID/UUID values, anchor IDs, semantic StableIds, serialized XML, source paths, or destination paths.

Use bounded output length and bounded path depth. Do not dump the entire tree or collect all differences.

### 3. Preserve code 30 through the public and desktop error paths

The underlying diagnostic shall begin with code 30 exactly as before, followed by the bounded structural detail. For example, the shape may be:

> 30: FDX contains a construct that cannot be projected safely; path=...; field=...; expected=...; actual=...

The example is illustrative, not a requirement to expose values. Keep the existing code 30 assignment; do not assign a new integer for the same normalized source-tree gate.

Extend the public error representation only as needed to carry owned bounded structural detail. If this requires removing Copy from an error type or introducing a dedicated source-tree mismatch variant, keep the change scoped and update affected call sites without changing unrelated error categories.

[`TransactionalSaveError::fmt()`](../../../crates/scriptcraft-core/src/fdx_storage.rs:297) and [`show_fdx_save_error()`](../../../apps/scriptcraft/src/main.rs:366) shall preserve the complete code 30 detail. Other numeric codes and non-codec errors remain unchanged.

Do not expose the crate-private WriterStage or debug formatting as a substitute for the intentional public diagnostic.

### 4. Keep production behavior unchanged

The expected tree, emitted bytes, re-read operation, semantic comparison, identity comparison, normalization, and atomic publication shall remain as they were before this order. Compute detailed difference metadata only after code 30 has already been established.

Do not reorder attributes or children, alter text segmentation, normalize additional metadata, skip SceneProperties, regenerate IDs, or convert any mismatch into a successful save.

## Regression coverage

Author [`v5_wo001_code30_structural_diagnostics.rs`](../../../crates/scriptcraft-core/tests/v5_wo001_code30_structural_diagnostics.rs) for public/transaction behavior and crate-internal unit tests for private field-level coverage.

Reference-backed cases:

1. Element local-name difference produces the correct path/category.
2. Attribute name/order/presence difference identifies attributes without exposing values.
3. ID-placement/attribute difference identifies the containing element and attribute category without exposing the UUID.
4. Namespace-binding difference is categorized without exposing unnecessary values.
5. Child missing/unexpected/reordered cases identify the correct path and counts.
6. Ordinary-text difference reports only length and whitespace classification, never content.
7. Empty versus whitespace-only text receives the correct classification after existing normalization rules are respected.
8. Code 30 begins with the integer 30 and retains the bounded detail through Save and Save As errors.
9. Desktop detail preserves the same path/category and remains within the bounded output policy.
10. Destination and live session remain unchanged after the numbered diagnostic failure.

Exhaustiveness-only synthetic cases:

- section name/kind/ordinal;
- source Type metadata;
- ownership and dependency confidence;
- anchor presence/category;
- CDATA, comments, and processing instructions;
- every full-equality-only field not covered above.

These synthetic cases prove diagnostic coverage, not Final Draft usage. Existing V4 numeric-code and V4-WO-001/V4-WO-002 behavior expectations remain unchanged.

Add a control showing that equal normalized trees produce no diagnostic difference and that no extra metadata is computed on successful saves.

## Durable documentation

Amend:

- [`technical-context.md`](../../../.forge/blueprints/technical-context.md:45) with the code 30 public detail schema, exhaustive field map, deterministic path rule, and privacy boundaries.
- [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md:403) with the bounded code 30 path/category requirement while retaining document-unchanged wording and all strict checks.
- [`fdx-fixes-v5.md`](../fdx-fixes-v5.md) with an execution log listing changed files, authored-but-unexecuted tests, and the exact user evidence needed next.

Do not update product preservation guarantees because no production round-trip behavior is changing.

## User-owned validation

Run the applicable formatting, lint, build, workspace, and focused V5-WO-001 tests. Then reproduce the existing failing Save on a disposable document and record the complete code 30 detail, operation sequence, and smallest available fixture.

Expected: the same save refusal occurs, but code 30 now identifies one deterministic structural field and path without exposing screenplay/source content. The detail should support selecting V5-WO-002's direct-versus-derived matrix and one conditional repair branch; it is not itself a Save fix.

## Handoff

After implementation, v5 remains in diagnostic mode until the user supplies the new code 30 detail. Use that detail to draft V5-WO-002 around the smallest direct or derived-candidate fixture. Do not dispatch a production repair solely from the integer 30 without the new structural category.

## Execution record — 2026-11-02

- Implemented exhaustive first-difference classification on normalized sections/nodes, with deterministic section/child-index traversal, bounded value-free public detail, and the existing strict code 30 refusal.
- Added `FdxError::SaveDiagnosticSourceTreeMismatch` carrying the detail through the transactional formatter and desktop error display; Save and Save As publication/session behavior was not changed.
- Authored core internal category/equality/privacy tests, the public codec plus Save/Save As transaction/privacy regression in `v5_wo001_code30_structural_diagnostics.rs`, and desktop detail projection coverage.
- Updated `technical-context.md`, `screenplay-writing-loop.md`, and the active v5 execution log.
- No tests, builds, formatting, lint, manual verification, implementation checks, reviews, or independent validators were run, per repository policy.
- Next user evidence: run the WO's applicable validation on the user's machine; reproduce the existing code 30 on a disposable document; capture the complete displayed diagnostic, operation (direct writer / Save / Save As), and smallest privacy-reduced fixture; confirm destination and live session remain unchanged. The resulting path/category, not integer 30 alone, selects V5-WO-002's direct-versus-derived matrix and conditional repair branch.

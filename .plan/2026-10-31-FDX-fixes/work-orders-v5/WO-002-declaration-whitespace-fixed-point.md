# V5-WO-002 — Declaration-adjacent whitespace fixed-point repair

- **Status:** Implemented; regressions authored but unexecuted; user validation pending
- **Depends on:** V5-WO-001 structural diagnostics and the confirmed mechanism in [`fdx-fixes-v5.md`](../fdx-fixes-v5.md)
- **Validation:** User-owned; no agent-run checks or independent validators

## Objective

Stop source-backed FDX serialization from adding one LF between the XML declaration and the preserved document tree on every Save. Restore direct serialization, Save, Save As, reopen, and repeated-save fixed-point behavior for existing documents while keeping the strict code 30 source-tree comparison intact.

## Confirmed defect

Two independent user documents produced code 30 at section 0/node 0: expected top-level whitespace length 1 and actual length 2. One report classified text.content; the other classified child.order. Both describe the same declaration-adjacent LF mismatch.

[`XML_DECLARATION`](../../../crates/scriptcraft-core/src/fdx.rs:41) already ends in LF. The reader validates but does not retain an XML declaration event, while it does retain ordinary top-level whitespace between the declaration and FinalDraft root. [`write_v6_inner()`](../../../crates/scriptcraft-core/src/fdx.rs:514) currently initializes output with the declaration including its LF at [`fdx.rs`](../../../crates/scriptcraft-core/src/fdx.rs:582), then writes the retained top-level LF again.

For an ordinary Final Draft source with one declaration-separating LF:

- expected preservation tree: one LF;
- emitted output: two LFs;
- re-read tree: two LFs;
- strict comparison: code 30 before destination replacement.

New preservation-free documents do not take the preserved-tree comparison branch, consistent with first saves succeeding and reopened saves failing.

## Selected fix

Separate the declaration's lexical text from declaration-to-root whitespace for source-backed output:

- when [`Project.preservation`](../../../crates/scriptcraft-core/src/lib.rs:467) is present, emit the standard declaration without adding a trailing LF of its own, then emit every retained top-level node unchanged;
- when preservation is absent, retain the existing canonical declaration-plus-LF behavior for newly generated documents.

This makes the retained source whitespace—or its absence—the only declaration-to-root separator in source-backed output. It handles sources with one LF, multiple LFs, spaces/tabs, no whitespace, or no original declaration without accumulating characters.

Do not remove or normalize retained top-level whitespace to make the comparison pass. The output itself must become fixed-point stable.

## Scope

1. Change only declaration/separator selection at the writer boundary.
2. Preserve all retained top-level nodes exactly, including whitespace, comments, processing instructions, and root position.
3. Keep generated-document declaration formatting unchanged.
4. Add direct codec and filesystem regressions for ordinary and repeated Save/Save As.
5. Exercise Final Draft references and minimal whitespace variants.
6. Update durable declaration/round-trip documentation and v5 execution records.

## Non-goals

- No weakening or special-casing of code 30 comparison to accept length differences.
- No removal of whitespace inside FinalDraft or other opaque elements.
- No SceneProperties, CharacterArcBeat, UUID, managed-empty-paragraph, Cast List, or pagination changes.
- No general XML pretty-printing or imported-source lexical normalization.
- No changes to semantic, identity, conflict, storage, or transaction rules.
- No speculative repair for any independent mismatch that remains after this defect is corrected.
- No agent-run tests, desktop automation, Final Draft automation, review, or validation.

## Implementation requirements

### 1. Select declaration separation by document provenance

At the output initialization in [`write_v6_inner()`](../../../crates/scriptcraft-core/src/fdx.rs:514):

- retain the existing full [`XML_DECLARATION`](../../../crates/scriptcraft-core/src/fdx.rs:41) for preservation-free generation;
- for source-backed projects, use the same declaration text without its trailing LF before iterating preserved nodes.

Use a clearly named constant/helper or bounded branch that makes the ownership distinction explicit. Do not trim retained nodes and do not condition behavior on the content of screenplay text.

The declaration must still occur exactly once at byte zero after any source BOM is intentionally not carried into output. No second declaration may be introduced from preserved content; the reader already excludes declaration events from the preservation tree.

### 2. Emit retained top-level nodes unchanged

Continue using [`write_node()`](../../../crates/scriptcraft-core/src/fdx.rs) for every expected top-level node in original order. Do not inject, remove, merge, or normalize declaration-adjacent whitespace at this stage.

After re-read, the actual top-level nodes should match the expected nodes under the existing normalization rules. The strict comparison at [`fdx.rs`](../../../crates/scriptcraft-core/src/fdx.rs:617) remains unchanged and must continue catching genuine differences.

### 3. Preserve generated-document formatting

New preservation-free documents shall still begin with exactly the current reference-compatible declaration followed by one LF and the canonical generated structure. Existing generated declaration, top-level order, indentation, and final-LF tests should not require relaxation.

### 4. Keep all failure and transaction safeguards

Do not alter candidate preparation, managed reconciliation, semantic/identity/tree comparisons, prepared-file validation, conflict detection, atomic replacement, Save As installation, session rebase, or document-unchanged error behavior.

A genuine later code 30 must still fail transactionally. This order repairs only the proven extra declaration LF.

## Regression coverage

Author [`v5_wo002_declaration_whitespace_fixed_point.rs`](../../../crates/scriptcraft-core/tests/v5_wo002_declaration_whitespace_fixed_point.rs).

Minimal direct-codec fixtures, each with valid managed Content:

1. Declaration plus one LF before FinalDraft: direct serialization emits one—not two—declaration-separating LFs and passes strict re-read.
2. Declaration plus multiple LFs: the exact count is retained without growth.
3. Declaration plus spaces/tabs or other legal top-level whitespace: retained nodes round-trip without an added LF.
4. Declaration immediately followed by FinalDraft with no whitespace: output remains valid and gains no forced separator.
5. Source without a declaration but with top-level whitespace: exactly one declaration is introduced and the original whitespace remains singular.
6. Source without a declaration and without top-level whitespace: output remains valid with exactly one declaration and no duplicate whitespace.
7. Top-level comment and processing-instruction generic controls retain their order and surrounding whitespace. These are parser/writer controls, not claims of Final Draft usage.
8. Serialize → reopen → serialize again produces the same declaration-adjacent bytes and no code 30.
9. New preservation-free output retains the existing declaration-plus-one-LF convention and one declaration total.
10. An independently injected source-tree mismatch still produces code 30, proving the check was not weakened.

Filesystem/reference scenarios:

- unchanged reopened [`reference2.fdx`](../../../.reference/reference2.fdx:1) Save succeeds on a disposable copy;
- unchanged reopened Save As succeeds without modifying the source;
- Save As output reopens and saves again without declaration whitespace growth;
- repeat the core Save/Save As/reopen control with [`arc copy.fdx`](../../../.reference/arc%20copy.fdx:1);
- retained screenplay, SceneProperties/arc content, IDs, and unrelated opaque source remain covered by existing assertions;
- injected storage/conflict failure retains destination and live session unchanged.

Where practical, compare only declaration-adjacent bytes and structural ownership assertions rather than requiring whole-file lexical identity for imported documents.

Update any prior test that incorrectly expects an added LF for imported source-backed output. Do not change generated-document expectations.

## Durable documentation

Amend:

- [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md:154) to distinguish generated declaration-plus-LF formatting from source-backed declaration separation using retained top-level nodes.
- [`technical-context.md`](../../../.forge/blueprints/technical-context.md:45) with the source-backed declaration separator ownership rule and the prohibition on accumulating top-level whitespace.
- [`fdx-fixes-v5.md`](../fdx-fixes-v5.md) with implementation details, authored-but-unexecuted tests, and remaining user-owned validation.

No product-overview change is expected; this restores the existing preservation guarantee rather than changing product scope.

## User-owned validation

Run the applicable formatting, lint, build, workspace, and focused V5-WO-002 tests. Then use disposable copies in ScriptCraft and Final Draft:

1. Open an existing Final Draft FDX, Save unchanged, close/reopen, and Save again.
2. Repeat through Save As and verify the source remains unchanged.
3. Inspect declaration-adjacent bytes after each save: no additional blank line or whitespace character accumulates.
4. Repeat with a ScriptCraft-created file after its first successful save and reopen.
5. Open ScriptCraft-saved output in Final Draft, save a separate copy, and reopen that copy in ScriptCraft.
6. Confirm JOHN/Arc 1 and empty managed Paragraph controls from v4 remain intact.
7. If another code 30 occurs, record the new complete path/category; it is a separate residual rather than evidence to weaken comparison.

Expected: ordinary existing-document Save and Save As complete, declaration-adjacent whitespace remains fixed-point stable, and Final Draft can open the saved files without introducing another ScriptCraft save failure.

## Handoff

The writer-boundary repair and requested direct-codec/filesystem regression coverage are authored. Repository policy prohibited agent-run tests, builds, formatting, lint, manual verification, implementation checks, reviews, and independent validators; none were run. User-owned validation remains as listed above. This order closes only the confirmed declaration-adjacent LF defect after that validation. If another failure remains, use the new diagnostic detail to select a fresh evidence-bound repair branch. Do not claim blanket FDX or Final Draft compatibility from this fix.

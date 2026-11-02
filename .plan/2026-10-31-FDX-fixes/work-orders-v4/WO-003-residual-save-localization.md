# V4-WO-003 — Residual reopened-save localization

- **Status:** Diagnostic localization implemented and authored coverage recorded; user-owned validation and residual mechanism identification remain open
- **Depends on:** V4-WO-001 and V4-WO-002 implementation records plus the user-owned reopened-save matrix
- **Validation:** User-owned; no agent-run checks or independent validators

## Objective

Identify the exact remaining writer/storage stage with a minimal reproducible fixture and privacy-safe structural diagnostics. Every diagnostic UnsupportedConstruct error reachable through the Save/Save As codec path shall begin its underlying diagnostic text with a stable integer mapped to the exact function call site that produced it. Do not make a speculative production fix beyond the diagnostic localization.

The user confirmed that the residual failure remains after V4-WO-001 and V4-WO-002, so the entry gate is met.

## Entry gate

Begin implementation only when the user supplies or confirms a residual failing scenario after V4-WO-001 and V4-WO-002. Record:

- whether the source originated in Final Draft or ScriptCraft;
- whether the operation was Save or Save As;
- whether the document was edited and whether scheduled derived refresh had time to run;
- the smallest available disposable FDX fixture;
- the exact public error and any already-known core reproduction;
- whether first serialization, reopen serialization, or a later save fails.

If every matrix scenario passes, add no diagnostic machinery solely for this order. Record closure in [`fdx-fixes-v4.md`](../fdx-fixes-v4.md).

## Current diagnostic limitation

[`write_v6()`](../../../crates/scriptcraft-core/src/fdx.rs:384) maps multiple failures to [`FdxError::UnsupportedConstruct`](../../../crates/scriptcraft-core/src/fdx.rs:64):

- original source graph divergence at [`fdx.rs`](../../../crates/scriptcraft-core/src/fdx.rs:388);
- managed element/source reconciliation at [`reconcile_project_elements()`](../../../crates/scriptcraft-core/src/fdx.rs:835) and [`reconcile_managed_paragraph()`](../../../crates/scriptcraft-core/src/fdx.rs:961);
- semantic re-read mismatch at [`fdx.rs`](../../../crates/scriptcraft-core/src/fdx.rs:444);
- identity-set mismatch at [`fdx.rs`](../../../crates/scriptcraft-core/src/fdx.rs:450);
- normalized source-tree mismatch at [`fdx.rs`](../../../crates/scriptcraft-core/src/fdx.rs:467).

The public message intentionally remains broad and privacy-safe, so the current user report cannot distinguish these gates.

## Scope for the confirmed residual failure

1. Add stable numeric diagnostic prefixes for every Save/Save As writer/storage UnsupportedConstruct call site, backed by one documented code-to-function/call-site map.
2. Add a crate-internal writer-stage representation and unit-level stage assertions for the minimal residual fixture.
3. For a source-tree mismatch, expose only bounded structural diagnostic metadata to core tests: node path, node kind, expanded element name, and relevant child/attribute counts.
4. Add an end-to-end reopened-save matrix to distinguish general writer, empty-content, derived-scene, arc-content, and unrelated opaque-data failures.
5. Record one concrete residual mechanism and a narrowly scoped follow-up recommendation.
6. Preserve serialization, strict checking, and transactional behavior during localization; only the approved diagnostic error representation changes.

## Non-goals

- No speculative production fix without a reproduced mechanism.
- No error-message redesign beyond the approved integer prefix and no screenplay/XML/path payload in diagnostics.
- No disabling or downgrading semantic, identity, source-tree, conflict, or storage checks.
- No blanket SceneProperties, UUID, formatting, or Final Draft compatibility rewrite.
- No agent-run tests, desktop automation, or Final Draft automation.

## Implementation requirements for the activated order

### 1. Add stable numeric call-site prefixes and private stage localization

Introduce one bounded diagnostic-code mapping for the Save/Save As codec path. Each integer shall identify one exact function and return/propagation call site, not merely a broad subsystem or a mutable source line number. Centralize the mapping rather than scattering undocumented numeric literals.

Use a backward-compatible bounded error representation where practical, such as a dedicated diagnostic UnsupportedConstruct variant or a writer-internal error converted at the public codec boundary. Existing non-diagnostic error categories remain unchanged. The underlying diagnostic text shall have exactly this form:

> INTEGER: FDX contains a construct that cannot be projected safely

The integer shall be the first character of that underlying diagnostic text. [`TransactionalSaveError::fmt()`](../../../crates/scriptcraft-core/src/fdx_storage.rs:297) and the desktop detail path shall preserve the prefix rather than stripping or relocating it. Document every assigned integer with its function and condition in [`technical-context.md`](../../../.forge/blueprints/technical-context.md:32) or a linked privacy-safe source map.

Cover top-level writer gates, managed reconciliation, new-document generation where reachable from Save, XML writing, and the storage semantic projection check. Do not leave a generic unnumbered UnsupportedConstruct escape on the Save/Save As path. Keep read/import compatibility behavior outside the diagnostic scope unless it directly supplies the failing Save re-read error.

Refactor the internals of [`write_v6()`](../../../crates/scriptcraft-core/src/fdx.rs:384) minimally so a crate-private helper can also return a bounded stage such as:

- project/preservation validation;
- original source graph comparison;
- managed source reconciliation;
- output re-read;
- semantic comparison;
- identity comparison;
- source-tree comparison.

The public [`write_v6()`](../../../crates/scriptcraft-core/src/fdx.rs:384) shall preserve existing failure/success and transaction behavior while returning the approved numbered diagnostic for UnsupportedConstruct save failures. Existing non-UnsupportedConstruct variants remain unchanged.

Place stage-specific unit assertions in the core module or another crate-internal test location that can access the private helper. Assert both the stable integer and its exact function/condition mapping. Do not add screenplay/source payload to the public API solely for diagnostics.

### 2. Add bounded structural difference reporting for tests

When the residual failure is the normalized source-tree comparison, compute the first difference only after the mismatch has already occurred. Diagnostic metadata may include:

- ordered child indexes forming a node path;
- element/text/CDATA/comment/processing-instruction node kind;
- namespace URI and local element name where necessary;
- child count and attribute count;
- identity domain and whether a required identity is missing or unexpected.

Do not include canonical screenplay text, arc text, attribute values, encoded image content, original XML, output XML, or filesystem paths. The purpose is to locate shape/ownership divergence, not inspect document contents.

Keep this diagnostic crate-private and test-oriented. Do not print it during ordinary application saves.

### 3. Build the minimal residual reproduction

Reduce the failing document in the following order, stopping at the first failure:

1. Action-only source with no Scene Heading, Cast List, or arc data.
2. Blank Action-only source with no Text child.
3. Scene Heading with empty SceneArcBeats.
4. Scene with JOHN/Arc 1 and no other Characters.
5. Disposable copy of [`arc copy.fdx`](../../../.reference/arc%20copy.fdx:1).
6. Disposable copy of the real-world reference.

For each stage, test unchanged direct serialization and the full detached save-candidate path. Distinguish a direct [`write_v6()`](../../../crates/scriptcraft-core/src/fdx.rs:384) failure from a later [`same_save_projection()`](../../../crates/scriptcraft-core/src/fdx_storage.rs:431) or session-rebase failure.

Author [`v4_wo003_residual_save_localization.rs`](../../../crates/scriptcraft-core/tests/v4_wo003_residual_save_localization.rs) for transaction-level coverage and use crate-internal unit tests for private stage assertions. Include Save and Save As where the residual involves filesystem publication.

### 4. Record the mechanism before proposing a fix

The implementation record must identify:

- the exact stage;
- the smallest fixture and operation sequence;
- the structural or identity difference, using privacy-safe metadata;
- whether V4-WO-001/V4-WO-002 changed the failure;
- the single ownership boundary implicated;
- the strict check that must remain intact;
- a proposed follow-up work-order scope or a reason no production change is justified.

Do not convert the diagnostic order into an open-ended rewrite. Any residual production fix requires a concrete amendment to [`fdx-fixes-v4.md`](../fdx-fixes-v4.md) and a new specific work-order boundary.

## Regression matrix for the activated order

Cover the residual fixture plus controls for:

1. Unchanged reopened Save.
2. Unchanged reopened Save As.
3. Ordinary non-Character edit followed by Save.
4. Scheduled/derived refresh followed by Save.
5. Save, reopen, and second Save of the first output.
6. Empty managed paragraph with no Text child.
7. Supported generated SceneProperties with retained arc text.
8. Deferred unsupported arc subtree beside a supported scene.
9. Unrelated opaque source and identity/reference controls.
10. Deliberate semantic, identity, and source-tree corruption still failing at the correct stage and returning the mapped integer.
11. Destination conflict and storage failure retaining their existing non-codec categories without a misleading writer code.
12. Diagnostic text beginning with the expected integer, the code map remaining unique/stable, and live-session/destination failure atomicity remaining unchanged.

Retain all V4-WO-001 and V4-WO-002 regression expectations; localization must not weaken them.

## Durable documentation

Document the bounded private stages and the complete public integer-to-function/call-site map in [`technical-context.md`](../../../.forge/blueprints/technical-context.md:32). Update the save diagnostic requirement in [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md:403) to require the numeric prefix while retaining privacy and document-unchanged wording.

Update [`fdx-fixes-v4.md`](../fdx-fixes-v4.md) with either:

- the closure evidence reference when no residual failure remains; or
- the exact residual mechanism, authored regression artifact, and proposed follow-up boundary.

A later production fix must amend requirements/blueprints according to its actual behavior.

## User-owned validation

The user runs all checks and supplies the residual reproduction. When diagnostics are implemented, the user runs the focused core/unit and transaction tests, repeats the smallest desktop/Final Draft scenario on disposable files, and reports the integer prefix from the failing Save/Save As detail.

Expected diagnostic outcome: one unambiguous failure stage and minimal fixture, identified by a stable integer prefix, with no screenplay/source payload exposed and no change in Save/Save As transactional behavior.

## Handoff

Close v4 only after the residual is either absent under user validation or recorded with a concrete follow-up mechanism. Do not claim the original universal save blocker is fixed solely because diagnostics identify its stage.

## Activation and implementation record — 2026-11-02

- **Entry gate:** Met. The user confirms that the Save failure remains after V4-WO-001 and V4-WO-002. The request explicitly activates diagnostic localization.
- **Residual reproduction supplied:** No concrete Final Draft/ScriptCraft fixture, operation sequence, original public error, or save phase was supplied. The implementation therefore does not identify or claim a residual production mechanism and makes no speculative production fix.
- **Diagnostic implementation:** Added stable numeric Save/Save As `UnsupportedConstruct` codes for every writer/storage return and propagation site. The centralized durable map is in [`technical-context.md`](../../../.forge/blueprints/technical-context.md). Private writer-stage and value-free source-tree/identity difference metadata are available only to core tests; ordinary application saves do not print them. Transactional and desktop detail paths preserve the `INTEGER: FDX contains a construct that cannot be projected safely` diagnostic text.
- **Authored coverage:** Added [`v4_wo003_residual_save_localization.rs`](../../../crates/scriptcraft-core/tests/v4_wo003_residual_save_localization.rs) with unchanged reopened-reference Save, Save As, and second-Save controls, plus exact code 23 Save/Save As failures and transaction atomicity; added crate-private stage, structural privacy, identity privacy, code uniqueness, and exact prefix/map assertions in [`fdx.rs`](../../../crates/scriptcraft-core/src/fdx.rs); added desktop prefix-preservation coverage in [`main.rs`](../../../apps/scriptcraft/src/main.rs). Existing writer-failure assertions were updated to expect their newly assigned numeric codes.
- **Unresolved evidence/blocker:** The exact user-observed residual failure and smallest failing disposable FDX are still needed to map an actual reopened-save failure to one of the now-numbered stages and to support a concrete follow-up recommendation. The synthetic code-23 case tests the diagnostic contract; it is not asserted to reproduce the reported failure.
- **Validation boundary:** No tests, builds, formatting, lint, manual verification, implementation checks, reviews, or independent validators were run. Authored tests and all reopened-save/interoperability validation remain user-owned. No success or closure claim is made.

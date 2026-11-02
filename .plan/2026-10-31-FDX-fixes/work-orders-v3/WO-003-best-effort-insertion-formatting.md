# V3-WO-003 — Best-effort formatting for imported managed insertions

- **Status:** Draft; implementation not started
- **Depends on:** [V3-WO-002](WO-002-context-scoped-title-page-identities.md) under sequential execution; technically independent of identity/scene behavior
- **Validation:** User-owned; no agent-run checks

## Outcome

Prevent cosmetic sibling-whitespace policy from blocking Save or Save As after a valid managed Paragraph insertion or split in imported FDX.

## Current defect

[`write_v6()`](../../../crates/scriptcraft-core/src/fdx.rs:385) collects inserted managed Paragraph IDs and calls [`format_inserted_paragraph_whitespace()`](../../../crates/scriptcraft-core/src/fdx.rs:724).

Only Content containing Paragraph elements plus structural whitespace is eligible for normalization, and effective XML space preservation disables it. If any inserted ID is not formatted, [`format_inserted_paragraph_whitespace()`](../../../crates/scriptcraft-core/src/fdx.rs:781) returns UnsupportedConstruct. A valid structural edit can therefore fail solely because beautification was unsafe.

## Scope

1. Retain existing LF/two-space sibling normalization for unambiguous imported Content made only of Paragraph elements and structural whitespace without effective XML space preservation.
2. When Content is mixed, opaque, or under effective XML space preservation, skip cosmetic normalization and emit the already reconciled managed Paragraph at its exact source-tree position.
3. Remove the formatting-only error produced when the formatted-ID set does not equal the inserted-ID set. Do not turn missing beautification into an unsupported document construct.
4. Keep the detached expected source tree identical to the intended emitted tree in each case. Strict semantic, identity, ordering, and opaque-source re-read comparison in [`write_v6()`](../../../crates/scriptcraft-core/src/fdx.rs:445) remains unchanged.
5. Preserve canonical Text, whitespace-only Text, CDATA, comments, processing instructions, mixed content, unknown elements, namespaces, source order, and XML space semantics. Do not globally pretty-print imported content.
6. Keep generated-document declaration, section order, indentation, blank lines, and final LF behavior unchanged.
7. Do not alter managed insertion/split identity allocation, Scene Heading attachment safe-stops, or transactional filesystem behavior.

## Regression coverage to author

Revise [`wo0042_fdx_structural_whitespace.rs`](../../../crates/scriptcraft-core/tests/wo0042_fdx_structural_whitespace.rs) and add focused cases in [`v3_wo003_best_effort_insertion_formatting.rs`](../../../crates/scriptcraft-core/tests/v3_wo003_best_effort_insertion_formatting.rs):

- Insert and split under effective XML space preservation, then Save/reopen with canonical text and sibling whitespace semantics unchanged.
- Repeat with comments, processing instructions, CDATA, non-whitespace mixed text, and non-Paragraph Content children.
- Verify unambiguous Paragraph-plus-structural-whitespace Content retains the existing readable normalization.
- Verify no formatting-only UnsupportedConstruct remains for an otherwise valid managed insertion.
- Retain strict re-read rejection for unintended role, text, identity, order, or opaque-node differences.
- Verify generated-document formatting and canonical Text tabs/LF/CR remain unchanged.

Replace the old authored refusal expectations rather than preserving formatting refusal as a product requirement. Authored cases are not validation results; the user performs all checks.

## Explicitly deferred

- Global imported XML pretty-printing or lexical byte identity.
- Structural editing of unsupported opaque content.
- Scene-derived data and title-page identity changes owned by preceding orders.

## Durable records

Update the imported insertion formatting requirement in [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md:157) and the formatting contract in [`technical-context.md`](../../../.forge/blueprints/technical-context.md:49). State that safe formatting is best-effort and ambiguous contexts are emitted without cosmetic normalization rather than refused.

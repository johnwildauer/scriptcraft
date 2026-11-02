# V3-WO-002 — Context-scoped identities for opaque title-page Paragraphs

- **Status:** Draft; implementation not started
- **Depends on:** [V3-WO-001.4](WO-001.4-nonblocking-derived-save.md) under sequential execution; technically independent of derived-scene behavior
- **Validation:** User-owned; no agent-run checks

## Outcome

Open real Final Draft documents that repeat Paragraph IDs in opaque title-page content, preserving every raw ID and element occurrence without weakening duplicate-identity checks for managed or referenced content.

## Confirmed reproduction and cause

[`real_script_example.fdx`](../../../.reference/real_script_example.fdx:244) repeats ae7f1040-c536-4085-aeec-464de6be3eac across title-page Paragraphs through [`real_script_example.fdx`](../../../.reference/real_script_example.fdx:272). These Paragraphs have no Type role and therefore remain opaque under [`preserve_node()`](../../../crates/scriptcraft-core/src/fdx.rs:2034).

[`collect_identities()`](../../../crates/scriptcraft-core/src/fdx.rs:2389) currently walks every Paragraph and inserts every ID into one global identity set. The second occurrence returns AmbiguousIdentity at [`fdx.rs`](../../../crates/scriptcraft-core/src/fdx.rs:2423), matching the reported open failure.

This is the first confirmed blocker in that reference; do not claim it is necessarily the only one without user validation.

## Scope

1. Make Paragraph identity collection context-aware in [`fdx.rs`](../../../crates/scriptcraft-core/src/fdx.rs). Distinguish opaque TitlePage Paragraph occurrences from managed body/title roles, derived nested arc Paragraphs, and explicitly referenced identities.
2. Permit repeated raw ID values on opaque TitlePage Paragraphs where no global identity/reference consumer requires uniqueness.
3. Preserve every occurrence, its source order, attributes, text/runs, and raw ID verbatim. Do not regenerate IDs, merge elements, or silently assign replacement identities.
4. Choose the smallest internal representation consistent with current preservation invariants:
   - Avoid registering those repeated opaque occurrences as unique global Paragraph definitions; or
   - Introduce an explicitly scoped occurrence representation if existing consumers require registration.
   Document the chosen boundary in the implementation handoff.
5. Keep strict duplicate/ambiguity rejection for managed body Paragraphs and any identity used by an explicit reference, board/image relationship, or another global lookup.
6. Keep existing internal semantic IDs and source anchors distinct from foreign raw IDs. No private FDX extension or source rewrite is permitted.
7. Do not globally disable duplicate checks, change ordinary UUID allocation, or broaden title-page editing.

## Regression coverage to author

Add focused core cases in [`v3_wo002_context_scoped_title_page_identities.rs`](../../../crates/scriptcraft-core/tests/v3_wo002_context_scoped_title_page_identities.rs):

- Open a minimal document with repeated opaque TitlePage Paragraph IDs and verify count, order, text, attributes, and raw repeated IDs survive Save and reopen.
- Open [`real_script_example.fdx`](../../../.reference/real_script_example.fdx:1) and exercise Save/reopen on a disposable in-memory candidate, checking canonical body text, title content, repeated raw IDs, and preserved board/list/opaque sections.
- Retain rejection for duplicate managed body Paragraph IDs and genuinely ambiguous referenced identities.
- Verify distinct opaque IDs and ordinary unique title IDs retain their existing behavior.
- After importing the real reference, verify HELEN MIRREN (V.O.) associates with derived HELEN MIRREN using the completed V3-WO-001 behavior.

If another independent real-reference blocker appears, record its exact typed category and minimal source shape; do not weaken unrelated reader checks merely to force the broad fixture to pass. Authored cases are not validation results.

## Explicitly deferred

- Title-page editing or position-based metadata inference.
- Relaxing identity requirements in managed body content or board/reference graphs.
- Final Draft application interoperability claims.

## Durable records

Update [`screenplay-writing-loop.md`](../../../.forge/requirements/features/screenplay-writing-loop.md:183) and [`technical-context.md`](../../../.forge/blueprints/technical-context.md:45) to distinguish raw repeated IDs in opaque title-page occurrences from globally unique managed/reference identities. Clarify in [`product-overview.md`](../../../.forge/requirements/product-overview.md:56) that imported IDs remain verbatim even when repeated in a non-global opaque context.

# FDX Fixes v3 — limitations and open questions

**Status:** Planning record only; no implementation work orders are approved here. V2 source handoffs describe authored implementation and authored tests, not verified behavior. User review, workspace checks, ScriptCraft manual validation, and Final Draft interoperability remain unperformed/unconfirmed. Do not treat any unknown below as a confirmed failure without a user-observed reproduction.

## Known boundaries reported by v2 implementation handoffs (source-derived; unvalidated)

- **Ambiguous scene-arc ownership after a separate reopen:** in-session creation ownership is not embedded in FDX. A stale arc entry may therefore be impossible to distinguish safely from imported/authored data after reopen. Reconciliation and the dependent Cast List refresh can safe-stop atomically rather than risk deleting authored beats or guessing ownership. This can block a later reconciliation/save; no durable ownership marker or safe removal policy is selected.
- **Ambiguous imported Content formatting:** inserted/split managed paragraphs can be refused with `UnsupportedConstruct` when Content has mixed/opaque structure or effective `xml:space="preserve"`. The implementation report says this is a deliberate formatting-safety boundary: do not normalize or guess; preserve the destination and live session on failure.
- **Save-error legibility at narrow widths:** v2 reports a more specific privacy-safe category in the existing one-line status message and left-aligns it. It adds no popup or new overflow interaction. Earlier source findings describe that one-line status as clipped; narrow-width clipping/readability therefore remains a reported UI limitation to confirm in user validation, not a claim that status handling is wholly broken.
- **PDF non-ASCII substitution:** the v1 FDX-fixes handoff and related typography record say existing PDF non-ASCII substitution was not repaired. V2.2 changed plain-text export only and explicitly left PDF/layout unchanged. Treat this as an existing recorded limitation unless user validation or a later source record establishes otherwise.
- **Random paragraph UUID limits:** source handoffs report UUID v4 allocations with bounded retry against current preservation/source IDs and a typed error on entropy failure or retry exhaustion. Existing imported IDs, including non-UUID IDs, remain verbatim. First-time IDs in an unsaved preservation-free document are random, so first-write bytes are not deterministic; serialization stability begins after IDs become established source state. There is no historical tombstone registry, so absolute non-reuse of a deleted historical UUID is not promised. Semantic IDs, source anchors, DocumentRef IDs, and board/image identities are outside this allocator.
- **Other preserved safety boundary:** Scene Heading structural split/join with attached scene payload may still safe-stop; existing unsupported or unknown FDX content is preserved rather than promised editable. Generated-document subset/order does not imply every Final Draft section or feature is supported.

## Unknowns awaiting user evidence

- Whether any or all v2 changes pass the user's workspace checks and practical ScriptCraft open/edit/save/reopen flows. Ten work orders have source-derived implementation reports and authored cases, but agents ran no validation; authored tests are not passing tests.
- Whether the original imported Action-to-Character save and lowercase/mixed-case typing reproductions are resolved in the user's real fixtures, including Save As and subsequent reopen.
- Whether scene arcs and Cast Lists stay synchronized for representative imported documents and how often ambiguous stale ownership prevents useful work. Do not infer a successful reconciliation from synthetic new-document cases.
- Whether ordinary imports, XML declaration/whitespace output, Unicode display-to-source mapping, plain-text export, UUID lifecycle, and privacy-safe error categories match actual user expectations across real documents.
- Whether Final Draft opens, edits, saves, and reopens any generated or ScriptCraft-round-tripped file without warnings, loss, or normalization. There is no compatibility claim; FDX is native direction, not blanket Final Draft parity.
- Whether the reported narrow status clipping is materially unreadable at the user's window sizes, or whether normal message geometry is sufficient. V2 intentionally did not add a popup/overflow affordance.
- How frequently the mixed/`xml:space` formatting refusal occurs in real imported FDX, whether a safe format-preserving solution exists, and whether the user prefers continued safe refusal or a separately designed supported subset.
- Whether scene-arc ownership can be made durable without altering opaque FDX content or making false ownership claims. No ownership format/default is selected.
- Whether UUID collision/entropy edge behavior and no-tombstone reuse scope are acceptable. Random collision is extremely unlikely but the bounded handling is source-reported, not validated; product expectations for historical non-reuse remain undecided.
- PDF non-ASCII substitution remains recorded as existing/unfixed; actual glyph coverage and desired remedy have not been confirmed by this initiative.

## Decisions and reproduction questions for a future planning round

1. Which user-observed cases failed, passed, were blocked, or were not run? Capture anonymized minimal fixtures, app/OS versions, operation sequences, sanitized error categories, and destination/session-preservation observations without screenplay text or private paths.
2. Do ambiguous stale scene arcs block a common editing/save workflow? What durable ownership evidence could safely distinguish ScriptCraft-derived entries from authored/imported entries while retaining unknown content? Which conditions must continue to safe-stop?
3. Can formatting-safe insertion/split support a larger imported Content subset without rewriting Text, opaque nodes, or effective `xml:space` semantics? Specify safe examples and a refusal boundary before implementation planning.
4. Does one-line status clipping warrant a separately approved UX change, given v2's explicit no-popup/no-new-overflow scope? Identify minimum width, control coexistence, accessibility, and privacy constraints.
5. Are UUID stability, first-write nondeterminism, bounded collision failure, preserved legacy IDs, and no historical tombstones acceptable? Is historical non-reuse a requirement or merely an unneeded guarantee?
6. What specific Final Draft versions and round-trip actions are in scope, and what observed evidence would justify any bounded compatibility claim? Keep interoperability separate from XML shape and ScriptCraft's own reopen behavior.
7. Is PDF non-ASCII substitution still a user-facing priority, and what sample characters/fonts/output would reproduce it? Avoid assuming a font-embedding or parity solution before product direction is recorded.

No implementation order is implied. Preserve prior v1/v2 records; future work should be scoped from user-observed evidence, not from authored-but-unrun cases or speculative failures.

# Work Order: WO-003 — Screenplay Writing Loop Requirements Cleanup

## Metadata

- **Status:** `draft`
- **Work-order kind:** `document-only`
- **Assignee:** To be assigned by the orchestrator after applicable gates.
- **Order/dependencies:** No hard predecessor WO; coordinate with [`WO-002`](WO-002-product-overview-cleanup.md) and the other document owners on shared decisions before closeout. The mapping below was proposed during planning and is now the approved execution baseline following the passing design-review follow-up and user authorization. A fresh requirements review of the resulting documents and independent per-WO validation remain separate; requirements review must pass before WO-004.
- **Initiative:** [`../documentation-cleanup-v1.md`](../documentation-cleanup-v1.md)
- **Generated filename:** `WO-003-screenplay-writing-loop-cleanup.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-003.md`](../validations/validation-WO-003.md) — future artifact.
- **Fixes:** none
- **Implementer session/agent ID:** pending dispatch
- **Dedicated work-order commit:** pending; exact full single-parent hash required when Git is usable.
- **Documentation-first relationship:** none; no dependent implementation WO is identified. The apparent desktop text-export gap is an initiative follow-up, not assigned code work.
- **Commit description convention:** `docs-first (pre-implementation): no dependent implementation WOs — screenplay writing loop requirements cleanup`; the label does not assert an implementation dependency.
- **Session exception:** none

## Summary

Make the feature requirements a cohesive, intent-first, independently traceable statement of desired writer-facing behavior. Preserve the existing contract and IDs except for the recorded snapshot/titlebar decisions and the approved duplicate-ID resolution below. Do not portray implementation evidence or WO history as requirements.

## In scope

- Edit only [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md), using the updated [`feature requirements template`](../../../requirements/features/feature-slug.md) and the [initiative's feature-specific findings](../documentation-cleanup-v1.md).
- Reorganize by area purpose, capability groups, constraints and related documents; preserve requirement-to-criterion relationships and durable intent. Integrate the substantive work-order refinement text into its appropriate acceptance criteria without work-order framing.
- Apply the user's recorded snapshot, lossy text-export, native-header, pasted-parenthesis, non-ASCII PDF, dirty-New consent, and schema-v1 persistence intent. Treat the ID disposition below as a proposed resolution for review, not an already-approved change. Identify unexpected material ambiguity to the orchestrator instead of inventing a change.

## Out of scope

- Editing the product overview, manifest, blueprints, historical initiatives/reviews/validations, process/templates, application source, tests or configuration; implementing or removing snapshot code; adding a text-export UI or successor WO.
- Changing remaining behavior, numerical bounds, safety guarantees, accessibility, semantic editing policies or platform limitations merely to shorten the document. No current-state inventory, validation claims, progress, backlog, WO history, or revision table in the durable requirements.
- Claiming native visual verification, operating-system notification delivery or fullscreen transition success; inventing a recovery replacement or a new titlebar interaction contract.

## Requirements

The [original feature requirements](../../../requirements/features/screenplay-writing-loop.md:37) are the comparison baseline. Preserve `REQ-WRITING-LOOP-001`–`006`, `NFR-WRITING-001`–`003`, and every existing acceptance ID and its substantive conditions except as explicitly mapped below:

- `001`: retain valid initial project/caret, role/canonical text, UTF-8-safe editing and undo/redo, full Enter/Tab matrix and no-op behavior, visual-line navigation, and bounded failure behavior. Integrate all dialogue split/ID/caret/rejection conditions; generated-parenthetical provenance and its distinct generated-pair Enter behavior; default-on cue casing and legacy behavior; and default-on parenthetical/strict-boundary, delimiter-repair, transactional, undo/no-op and legacy-decoding conditions. Pasted parentheses remain intact during paste and thereafter follow normal document Tab/Delete handling; do not protect pasted delimiters based on provenance or conflate that handling with generated-pair Enter cleanup. Preserve settings/schema compatibility conditions without requiring WO labels.
- `002`: retain fixed-profile deterministic pagination, source mapping, role-aware measures/keeps, oversized-element safety, and valid-boundary caret/hit-test round trips.
- `003`: retain native schema-v1 serialization/reopen, saved content and transient-state exclusion, failure safety and dirty state in `.1`–`.3`; default legacy fields as recorded and safely reject malformed or unsupported schema versions. Do not promise cross-version migrations. Retire snapshot criterion `.4` without replacement or ID reuse and remove snapshot-related story, overview, terminology and prose; do not invent another recovery promise.
- `004`: retain fixed-layout PDF that preserves non-ASCII characters, native-source/dirty-state safety, and lossy plain-text export with its roles/IDs/page-layout loss communication. Do not reinterpret text as lossless or remove its requirement because the desktop route appears absent.
- `005`: preserve default toolbar order, action routing, accessibility/hover semantics, role selection and rejection, width/allocation/overflow and cursor ownership. **Approved execution mapping:** retain toolbar width/overflow as `AC-WRITING-005.5`; restore the duplicated cursor-owner clause to its historically supported `AC-WRITING-005.7`; use `.6` only for the user-permitted narrow OS-owned native macOS header/content criterion. The historical app-owned strip/maximize criterion is retired and is not equivalent. Do not promise a particular OS double-click outcome or native visual result.
- Also preserve the recorded New safeguard in its appropriate existing criterion: a dirty session must not be replaced by New without the writer's explicit informed consent. Do not prescribe a dialog, action sequence, or alternate recovery policy.
- `006`: preserve all `.1`–`.13` status, error/privacy, overflow and visible-page rules; zoom values, fixed page size, invariants and geometry; footer stability and clipping; pinch and scale fallback/platform bounds; appearance modes and contrast; timer lifecycle and best-effort notification; and reported-state Focus Mode limitations. Recast implementation-specific language only when its observable, independently testable meaning and numeric bounds remain intact.
- Preserve the three NFRs, the existing non-goals, and useful product-overview/feature-blueprint links. Keep technical mechanisms only where necessary to make the existing durable contract precise; do not turn architecture or state evidence into product requirements.

## Blueprints

- [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md), [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md), and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md) are read-only coordination context. Their snapshot and/or titlebar descriptions may need separate owner reconciliation. Do not change them or claim they are already consistent.

## Implementation plan

1. Compare the whole original feature file with the area template and the preservation inventory above. Reorganize into established-state purpose, capability groups, constraints, and related documents. Keep each requirement and AC uniquely associated and independently testable; retain all semantic conditions, data-safety obligations, numeric thresholds, exclusions and platform qualifications.
2. Remove document status/owner/date, observed/inferred/current evidence, WO-specific headings and approvals, evidence placeholders, obsolete risk inventory and revision history. Translate the three in-body refinements and the trailing parenthetical/boundary refinement into the relevant existing editing criteria or clearly associated constraints; do not drop details when moving them. Remove obsolete implementation-specific phrasing where it is not part of a necessary testable contract.
3. Apply the recorded user decisions and approved ID mapping exactly as described in Requirements: retire `AC-WRITING-003.4` without reuse; keep width/overflow at `.5`; restore cursor ownership to historical `.7`; and use `.6` only for the non-equivalent native-header criterion. The mapping passed design review and is included in the user's direct execution authorization; preserve historical references. If additional renumbering or material behavior change is necessary, stop and report it.
4. Check the original and revised file section by section against the preservation inventory, decision record and acceptance tests. Maintain no history or ID mapping table in the durable requirements; the initiative and this WO carry the mapping. If another material intent or current-state conflict is discovered, report it to the orchestrator; do not silently align requirements to code or extend this work order.

## Files and systems

- **Create:** none as part of the document edit.
- **Update:** [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md) only.
- **Avoid changing:** other durable documents, historical artifacts, app/core source, tests, configuration, process and templates.

## Validation and E2E acceptance tests

- **Document checks:** Compare the full before/after document against all preservation bullets and the four refinement blocks, including conditions, values, exceptions, and explicit platform limitations. Enumerate all requirement, acceptance, and NFR IDs and verify uniqueness and the exact mapping above. Check Markdown structure and resolve retained relative links; run `git diff --check`. Record results and any unavailable or failed check. Run project-policy checks supplied by the orchestrator when applicable; do not claim fresh runtime validation from a documentary comparison.
- **Acceptance:** Given the original file, recorded user decisions, the mapping approved at the design/approval gate, and the template, when an independent reader compares the scoped diff, then all in-scope durable behavior and IDs are preserved except the approved retirement and mapping; each acceptance ID occurs once; width/overflow remains `.5`, cursor ownership is `.7`, and the user-permitted non-equivalent native-header criterion uses `.6`, with historical traceability intact. Snapshot promises are gone without weakening save safety; native persistence specifies schema-v1 support, defaulted legacy fields and safe rejection without promising migrations; pasted parentheses remain intact through paste and follow normal Tab/Delete behavior, distinct from generated-pair Enter; non-ASCII PDF preservation and informed consent before dirty-New replacement are included; lossy text export and its warning remain; and the requirements contain no lifecycle, evidence, WO narrative, revision history or implementation-gap claims. The accepted mapping is not required to remain merely proposed after the gate passes.
- **Independent validation:** After applicable review, approval, and execution, a fresh validator receives this WO and its exact dedicated commit hash and checks that commit's scoped documentation diff against these criteria. No app-source inspection or dependent implementation is required for document-only validation; no validation pass is implied by this draft.

## Durable-document impact

- **Requirements impact:** `update` — only the feature requirements file; `REQ-WRITING-LOOP-001`–`006`, their ACs and `NFR-WRITING-001`–`003` are affected by reorganization and the specified dispositions.
- **Blueprint impact:** `verify unchanged` for this WO's file boundary; no blueprint edits are authorized. Related snapshot/titlebar references need separate orchestrator-coordinated disposition and may remain temporarily inconsistent.
- **Rationale and evidence:** The [initiative finding](../documentation-cleanup-v1.md) records the user's decisions, historical ID evidence, native-source boundary and apparent text-export integration gap. The gap is not proof of desktop availability and is not a license to remove the retained export intent.
- **Authorized documentation changes:** Only the feature requirements changes described above; no dependent implementation WO is identified. Orchestrator records any continuing cross-document or document/code gap in the initiative handoff, not in durable requirements.

## Completion checklist

- [ ] Feature-only edit compared against every original requirement, refinement, constraint and exclusion; exact ID/retirement/reuse mapping checked.
- [ ] Template, Markdown/link, diff and applicable policy checks recorded, including failures and unavailable checks.
- [ ] Orchestrator records dispatch/completion identifiers, changed files, evidence and exact dedicated commit hash, or the applicable no-change/Git disposition.
- [ ] Fresh independent validator receives this WO and exact hash; drafting or implementation alone does not complete validation.
- [ ] Orchestrator coordinates WO-002 and the other document owners and carries the text-export gap forward without expanding this assignment.

## Handoff

Provide the approved WO, original feature file, area template, initiative's feature-specific finding and both user decisions, plus applicable checks. Orchestrator owns cross-document ordering and separate review/approval gates. Preserve unrelated changes and stop if they cannot be isolated safely; drafting does not authorize execution.

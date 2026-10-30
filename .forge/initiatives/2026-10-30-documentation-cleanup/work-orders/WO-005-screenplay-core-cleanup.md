# Work Order: WO-005 — Screenplay Core Blueprint Cleanup

## Metadata

- **Status:** `draft`
- **Work-order kind:** `document-only`
- **Assignee:** To be assigned by the orchestrator after applicable gates.
- **Order/dependencies:** No hard predecessor WO. Coordinate the parenthetical decision with WO-003, shared snapshot/export decisions with WO-002–004 and the other blueprint owners, and the PDF and cache gaps with their owners. Independent design review and required human design approval precede execution. No dependent implementation WO is identified.
- **Initiative:** [`../documentation-cleanup-v1.md`](../documentation-cleanup-v1.md)
- **Generated filename:** `WO-005-screenplay-core-cleanup.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-005.md`](../validations/validation-WO-005.md) — future artifact.
- **Fixes:** none
- **Implementer session/agent ID:** pending dispatch
- **Dedicated work-order commit:** pending; exact full single-parent hash required when Git is usable.
- **Documentation-first relationship:** none; no dependent implementation WO is identified. The orchestrator owns the recorded code and cross-document gaps and any future disposition.
- **Commit description convention:** `docs-first (pre-implementation): no dependent implementation WOs — screenplay core blueprint cleanup`; the label does not assert a code dependency.
- **Session exception:** none

## Summary

Reconcile the Screenplay Core capability blueprint into a durable description of intended architecture and contracts in established-state voice. Preserve substantive safety, compatibility and behavior, applying only the explicit user decisions below. Remove initiative-specific progress, evidence and history rather than presenting current implementation as the target.

## In scope

- Edit only [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md), using the updated [component template](../../../blueprints/components/component-slug.md) and the initiative's [core-specific findings and decisions](../documentation-cleanup-v1.md).
- Describe the headless semantic model, editing/session, layout, persistence and export capability, component relationships, invariants, interfaces and only material architecture decisions with rationale. The desktop consumes the core; the core has no widget dependency.
- Preserve the technical contracts enumerated below, while removing the obsolete or conflicting claims identified here. Apply recorded decisions about recovery, parentheses, native versions, and exports without silently changing other intent.

## Out of scope

- Editing requirements, other blueprints, the manifest, source, tests, configuration, process/templates or historical artifacts; implementing, deleting recovery code, fixing PDF or cache behavior, or creating successor WOs.
- Including WO-specific text, lifecycle status, owner/date, test-run or benchmark evidence, evidence placeholders, progress, revision history, unsupported design rationale or implementation-gap narratives in the durable blueprint. Do not make an ADR out of a temporary implementation observation.
- Inventing a recovery replacement, cross-version migration policy, text-export UI, new platform or titlebar interaction, broader font support, or unverified native/OS behavior. The earlier user decision retains the OS-owned macOS header, not an app-owned title strip; chrome belongs to the desktop boundary, not the core.

## Requirements and blueprint context

- [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md) is the sole edit target and before-state baseline. [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md) supplies semantic editing, deterministic layout, native-save safety and export intent; its older snapshot and parenthetical refinement wording is superseded for this assignment by the user decisions below. The orchestrator must coordinate its separate reconciliation rather than altering it here.
- [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md), and [`technical-context.md`](../../../blueprints/technical-context.md) are read-only boundary/coordination context. They may remain temporarily inconsistent until their owners act.
- Supporting inspection belongs to the initiative, not the durable blueprint: [semantic model/profile](../../../../crates/scriptcraft-core/src/lib.rs:154), [editing](../../../../crates/scriptcraft-core/src/editing.rs:142), [layout/cache](../../../../crates/scriptcraft-core/src/layout.rs:63), [persistence](../../../../crates/scriptcraft-core/src/persistence.rs:56), [exports](../../../../crates/scriptcraft-core/src/exports.rs:46), [parenthetical policy tests](../../../../crates/scriptcraft-core/tests/wo007_parenthetical_policy.rs:237), and [paste safety tests](../../../../crates/scriptcraft-core/tests/wo002_paste_safety.rs).

## Required contracts and decisions

1. **Model and safety.** Preserve explicit seven screenplay roles, ordered canonical UTF-8 text, versioned project metadata/title-page/elements/profile and stable opaque unique IDs, independent of position, appearance or page geometry. A new project contains exactly one empty Scene Heading and a valid initial caret at element zero, byte zero. Preserve checked UTF-8 boundaries, bounded structure and serialization: at most 10,000 elements, 262,144 text bytes per element, and 1,048,576 native JSON bytes. Mutations and bounded paste are transactional, a paste is one undo entry, newlines/whitespace remain in the active element rather than inventing roles, and failures preserve project, selection, history, revision and dirty state. Preserve bounded collision-checked split IDs, undo/redo and typed/recoverable failures.
2. **Editing policies.** Preserve explicit Enter/Tab policies, unlisted Tab no-ops without literal tabs or history, and the boundary-sensitive Dialogue split: end goes to Action; interior valid collapsed boundary preserves the original ID/prefix and inserts a distinct bounded ID/suffix as Dialogue, with the caret at the new element start. Rejected selections, invalid boundaries and capacity failures preserve the session. Preserve default-on cue uppercase for Scene Heading and Character at edit/role-transition boundaries, Unicode transformed-prefix selection mapping, legacy-missing default, no rewrite on open and other-role casing preservation.
3. **Parentheticals.** Preserve default-on persisted automatic parentheticals and strict transition boundaries, legacy-missing defaults, malformed-type rejection and profile round-trip. When automatic generation applies, generated empty pairs have an interior caret and transient stable-ID provenance, included in undo/redo but not native persistence; successful content-changing edits invalidate the mark and failed edits do not. Describe the existing Enter-specific generated-pair cleanup, before-opening insertion, supported delimiter repair, strict-boundary no-ops and undo/rollback conditions without asserting all Enter/Tab cleanup requires provenance. Per the user's decision, paste preserves parentheses at paste time; afterward pasted delimiters are normal document text. With automatic parentheticals enabled, Tab handles delimiter-only `(`, `)` and `()` as an empty Dialogue transition regardless of provenance; Delete handles delimiter-only Parentheticals with previous-element or first/only-element fallback, as specified by the existing policy. Preserve nonempty authored content, distinctions for Enter, settings-off behavior and transactional undo; do not claim all authored/partial delimiters are permanently immune to explicit commands.
4. **Fixed layout and mapping.** Preserve the versioned bounded page profile and geometry: US Letter 612 × 792 points, left/right/top/bottom margins 108/72/72/72, seven-point character cells, twelve-point line advance and boundary gap, 55 nominal lines, and role boxes scene/action/note 108–540, Character 259–434, Parenthetical 216–396, Dialogue 180–396 and right-aligned Transition 432–540. New projects default to Courier Prime at 12 points; preserve supported existing Courier and Courier New profiles without rewriting. Remove the obsolete earlier claim that new projects default to Courier. Retain title-page separation, role-aware wrapping and keep relationships, safe oversized-element escape, LF/CRLF/CR handling, source byte ranges, caret/hit testing and deterministic project/profile inputs independent of viewport scaling. Incoming Dialogue or Parenthetical gets zero additional body boundary gap; other incoming roles use the profile gap; wrapped lines and title pages are excluded. Preserve document-global nearest body line, earlier-line/lower-page tie-breaking, horizontal and document-end clamping, UTF-8-safe mappings and title-page exclusion.
5. **Cache and integration.** State that cached derived layout is immutable and valid only for the current semantic project and complete profile; a new revision/profile or project/session replacement must not reuse stale geometry. Viewport scaling is presentation-only. Desktop paint, hit testing, caret geometry and visual-line navigation consume appropriate shared layout; the preferred horizontal goal is transient and not persisted. PDF consumes the fixed print layout, not the viewport. Do not assert revision plus profile alone identifies different sessions: the initiative records a source-level replacement risk, not an authorized fix.
6. **Native persistence and exports.** Preserve schema-version-1 native semantic round trips for roles, IDs, metadata, text and page profile, legacy-missing profile policy defaults, bounded UTF-8 parsing, and safe typed rejection of malformed or unsupported versions. Do not promise cross-version migrations. Native save prepares bounded data and uses atomic destination replacement; failed preparation/write/replacement preserves the last valid named destination and unsaved/dirty state; mark clean only after successful replacement. Exclude caret, layout, undo and transient provenance from native content. Remove snapshot-recovery promises without weakening these guarantees or inventing a substitute. PDF and deliberately lossy text are noncanonical outputs that do not replace or mark clean the native project; PDF preserves non-ASCII characters and uses fixed layout, while text communicates loss of roles, stable IDs and page layout. The known PDF and desktop text-route gaps belong to the initiative, not the target blueprint.

## Implementation plan

1. Compare every section of the original blueprint, including trailing WO refinements, against the preservation inventory above. Reorganize into Capability, Composition, Contracts and Decisions as in the template; component blocks or concise relationships should clearly distinguish semantic model, session, layout, persistence and export ownership and the desktop integration boundary.
2. Integrate lasting behavior from the older WO-labeled sections into coherent contracts with conditions, values and exceptions intact. Remove status, owner/date, source/test evidence, timing/benchmark descriptions, contradictory or obsolete foundation claims, placeholders, WO framing and revision history. Keep only decisions that materially clarify target structure; omit the outdated speculative crate-splitting ADR unless its rationale is independently grounded in the supplied target.
3. Apply the exact user decisions and the cache-correctness invariant above. Preserve the distinction between generated Enter cleanup and the normal later Tab/Delete handling of pasted delimiters. Do not infer snapshot code deletion, implementation success, a PDF fix or a text-export UI. If another material intent or current-state conflict is uncovered, stop and report it to the orchestrator for the user rather than guessing or modifying another file.
4. Check the final document and exact scoped diff against every contract and exclusion below. Orchestrator coordinates other document owners and maintains material target/code lag in the initiative execution handoff, not the durable blueprint.

## Files and systems

- **Create:** none as part of the document edit.
- **Update:** [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md) only.
- **Avoid changing:** all other durable documents, source, tests, assets, configuration, process/templates and historical artifacts. The separately user-authorized PDF TODO in the working tree is not part of this document-only WO or its commit.

## Validation and acceptance

- **Document checks:** Compare original and revised blueprint section by section, including every trailing refinement, against all six numbered contract inventories, decisions and exclusions; record any omission or mismatch. Check Markdown structure and retained relative links, inspect the exact scoped diff, and run `git diff --check`; report failed or unavailable checks. Run separately applicable project-policy checks supplied by the orchestrator. Application tests are not inherently required for this document-only assignment, and documentary inspection is not a runtime or native smoke pass.
- **Acceptance:** Given the original blueprint, updated component template and initiative decisions, an independent reader can identify the capability's ownership and interfaces and verify all six contract groups, exact geometry and bounds, policy exceptions and save/rollback guarantees. Snapshot promises and migration claims are absent; pasted text follows the specified later Tab/Delete behavior; PDF preserves non-ASCII in the target; text export is explicitly lossy; and cache correctness includes project replacement. Courier Prime is the unambiguous new-project default. There are no WO narratives, implementation evidence, obsolete status, revision history, unsupported decisions, invented features or other-file edits.
- **Independent validation:** After applicable review, approval and execution, a fresh validator receives this WO, the assigned before/after documents and its exact dedicated single-parent commit hash, checks that exact commit against the acceptance criteria and records the result in the linked validation artifact. Document-only validation does not inspect application source or block on known dependent code gaps. Drafting is not validation, approval or execution.

## Durable-document impact

- **Requirements impact:** `verify unchanged` — requirements are read-only; the parenthetical refinement requires separately coordinated reconciliation.
- **Blueprint impact:** `update` — only the screenplay-core component blueprint; all other blueprints are read-only.
- **Affected documents/IDs:** Core blueprint; coordinate existing writing-loop requirements and related blueprints without changing IDs here.
- **Rationale and evidence:** The initiative records prior and newly resolved user decisions, the PDF substitution, the possible stale-layout case, and the separate text-export integration gap. None authorizes implementation.
- **Authorized documentation changes:** Only the target described above; no dependent implementation WO is identified. Orchestrator records cross-document and code/target gaps in the execution handoff when applicable.

## Completion checklist

- [ ] Core-only edit compared against the complete original and all six preservation inventories and decisions.
- [ ] Markdown/link, scoped diff and applicable policy checks recorded; unavailable or failed checks reported.
- [ ] Orchestrator records dispatch/completion identifiers, changed files, evidence and exact dedicated commit hash, or applicable no-change/Git disposition; unrelated source TODO excluded.
- [ ] Fresh independent validator receives this WO and exact hash; drafting or implementation alone does not complete validation.
- [ ] Orchestrator coordinates WO-003 and other owners and carries recorded gaps forward without expanding this assignment.

## Handoff

Provide the approved WO, original core blueprint, updated component template, initiative core findings and user decisions, and applicable checks. Preserve unrelated work and stop if the scoped edit cannot be isolated safely. Drafting does not authorize execution, review, approval or a code fix.

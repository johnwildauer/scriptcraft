# Work Order: WO-007 — Screenplay Writing Loop Feature Blueprint Cleanup

## Metadata

- **Status:** `draft`
- **Work-order kind:** `document-only`
- **Assignee:** To be assigned by the orchestrator after applicable gates.
- **Order/dependencies:** No hard predecessor WO. Orchestrator supplies the recorded decisions and coordinated requirement-ID mapping; coordinate WO-002–006 and the manifest owner without assuming their drafts have executed. Applicable requirements review, independent design review and required human design approval precede execution. No dependent implementation WO is identified.
- **Initiative:** [`../documentation-cleanup-v1.md`](../documentation-cleanup-v1.md)
- **Generated filename:** `WO-007-screenplay-writing-loop-blueprint-cleanup.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-007.md`](../validations/validation-WO-007.md) — future artifact.
- **Fixes:** none
- **Implementer session/agent ID:** pending dispatch
- **Dedicated work-order commit:** pending; exact full single-parent hash required when Git is usable.
- **Documentation-first relationship:** none identified; the target blueprint can lead subsequent work, but no dependent implementation WO is authorized or identified. The orchestrator owns the recorded target/code gaps.
- **Commit description convention:** `docs-first (pre-implementation): no dependent implementation WOs — screenplay writing loop feature blueprint cleanup`; the label does not assert a code dependency.
- **Session exception:** none

## Summary

Reconcile the writing-loop feature blueprint into a durable technical composition of the desired system in established-state voice. Preserve substantive contracts and stable traceability, apply the recorded user decisions, and keep implementation evidence and initiative history outside the blueprint. The target leads work orders; it is not proof that current code already satisfies it.

## In scope

- Edit only the [writing-loop feature blueprint](../../../blueprints/features/screenplay-writing-loop.md), comparing its complete original, including trailing sections, with the [feature-area template](../../../blueprints/features/feature-slug.md) and [initiative feature findings](../documentation-cleanup-v1.md#writing-loop-feature-blueprint-analysis-and-wo-007-handoff).
- Describe the desktop/core composition and cross-boundary flows for semantic editing, deterministic layout, native persistence, exports, toolbar, status and transient presentation. Connect all six existing feature requirements and the applicable acceptance criteria at useful capability granularity, without copying detailed component or container inventories.
- Integrate durable conditions from the existing models, contracts, decisions and trailing dialogue and parenthetical refinements. Apply the recorded user decisions and coordinated ID treatment below.

## Out of scope

- Editing any other durable document, existing draft, process/template, historical record, application/core source, tests, assets, or configuration; deleting snapshot code, fixing behavior, creating implementation successors, conducting review or granting approval.
- Carrying WO-specific notes, status/owner/date, current-state evidence, benchmark figures, progress, placeholders, revision history, code-gap reports or unverified native observations into the blueprint. Do not turn an implementation observation into an architectural decision or weaken an established contract to make the document shorter.
- Inventing a recovery substitute, migration between schema versions, text-export UI, dirty-New consent interaction, platform support, native titlebar gesture, notification delivery, fullscreen completion or PDF encoding mechanism. Do not infer runtime or native visual verification.

## Requirements and traceability

- The [feature requirements](../../../requirements/features/screenplay-writing-loop.md:37) are read-only context. Cover `REQ-WRITING-LOOP-001`–`006` and applicable `AC-WRITING-001.1`–`.5` including `.4a`/`.4b`, `002.1`–`.4`, `003.1`–`.3`, `004.1`–`.3`, toolbar criteria, and `006.1`–`.13`; connect `NFR-WRITING-001`–`003` to bounded failure, headless testability and deterministic output. Use requirement-level links where acceptance-level duplication would be noisy, but make each capability's realization and ownership independently traceable.
- Apply the [recorded decisions and approved execution mapping](../documentation-cleanup-v1.md#writing-loop-feature-blueprint-analysis-and-wo-007-handoff), which supersede unreconciled recovery, parenthetical and title-strip prose. `AC-WRITING-003.4` is retired without replacement or reuse. WO-003 owns the requirements changes: `.5` remains width/overflow and `.7` is cursor ownership; `.6` is used only for the user-permitted, non-equivalent native-header criterion. If WO-003's corrected requirements have not yet been supplied, describe cursor ownership without assigning a false identifier; do not modify IDs or history in this WO.
- The [product overview](../../../requirements/product-overview.md) is read-only context. Orchestrator coordinates the updated parenthetical, PDF Unicode and dirty-New decisions with requirements owners; this WO does not assume those documents are already reconciled.

## Blueprint and implementation context

- The [feature blueprint](../../../blueprints/features/screenplay-writing-loop.md) is the sole target; the [technical context](../../../blueprints/technical-context.md), [core component](../../../blueprints/components/screenplay-core.md), and [desktop container](../../../blueprints/containers/scriptcraft-desktop.md) are read-only composition references, independently subject to WO-004–006. Link documents with resolvable relative paths; do not rely on the original component-name fragments, which are fenced-block names rather than headings, unless corresponding stable anchors have actually been supplied.
- The initiative records source observations and gaps. In particular, the [core session](../../../../crates/scriptcraft-core/src/editing.rs:142) owns semantic edit/transition behavior; [core exports](../../../../crates/scriptcraft-core/src/exports.rs:46) and desktop file/platform adapters have distinct responsibilities. Source is context, not proof the target is met or a reason to silently alter intent.

## Required composition and contracts

1. **Shared path and ownership.** Describe the local desktop as the user-input, IME, window, file-selection, presentation and platform boundary. It delegates semantic changes to the reusable headless core's project and transactional session; the session owns explicit role-transition policy, not an independent desktop role-policy component. The core derives shared fixed-profile page geometry and source mappings, owns native serialization/persistence and supplies export adapters; the desktop consumes results and routes file and status interactions. Do not imply the core is a separate runnable service or that the desktop owns a second canonical text buffer.
2. **Semantic model and editing.** Preserve a versioned project with stable opaque IDs, metadata, title-page content, page-profile inputs and ordered elements with seven explicit roles and canonical UTF-8 text; transient caret/selection, geometry and undo are not required native content. A new project starts with one empty Scene Heading and a valid element-zero/byte-zero caret. Checked bounded transactional edits, UTF-8-safe positions, undo/redo and failure rollback protect project/session state. Preserve explicit Enter/Tab behavior and no-op boundaries, cue casing and legacy defaults, plus the Dialogue interior split: original ID/prefix retained, bounded distinct ID/suffix inserted as Dialogue, caret at the new start; end transitions to Action and rejected operations leave state unchanged. Preserve generated-pair Enter cleanup and transient provenance, undo/redo and invalidation. Pasted parentheses are preserved at paste and thereafter receive normal document Tab/Delete handling, including delimiter-only transitions/deletion and applicable fallback; do not assert blanket permanent provenance protection. Delegate detailed matrices and numeric limits to the core blueprint rather than duplicating them.
3. **Layout, font and presentation.** Preserve the versioned fixed screenplay profile and role boxes, source-byte mappings, role-aware wrapping/keep and safe oversized-element behavior, body role-pair gap policy, title-page separation, caret/hit testing and viewport-independent fixed print geometry. Incoming Dialogue and Parenthetical have no extra body boundary gap; other incoming body roles use the profile boundary gap, excluding wrapped lines and title-page boundaries. New projects default to Courier Prime; supported stored Courier/Courier New profiles retain their identity. Shared layout feeds screen navigation, hit/caret/selection and PDF; cached geometry must represent the current project and complete profile after changes and session replacement. Zoom, appearance, timer, focus mode and status overflow are transient desktop state, not project or print-layout data.
4. **Native save and export.** Preserve schema-v1 semantic round trips, legacy-missing field defaults and typed safe rejection of malformed/unsupported versions, without a cross-version migration promise. Native save prepares bounded data and atomically replaces the named destination; failed open/save/export preserves the valid session and native destination, and path/clean state changes only after successful save or open as applicable. Remove snapshot-recovery promises without inventing a replacement or deleting code. PDF uses fixed print layout and preserves non-ASCII characters; text is a deliberate lossy noncanonical output communicating lost roles, stable IDs and page layout. Neither export silently replaces the native source nor marks it clean. Do not freeze Type1-only mapping or non-embedding if it conflicts with Unicode preservation; do not invent a text-export route.
5. **Desktop integration.** New must not replace a dirty session without the writer's explicit informed consent, without specifying the UI. Preserve toolbar action/session routing, Elements role selection, width/overflow and cursor ownership; the native macOS header remains OS-owned with toolbar below it in content, not an app-owned title strip. Preserve status as a privacy-conscious projection of session, operation/errors and fresh visible-page geometry; it must not display the native path. Accessible toolbar/status controls and transient zoom, appearance, timer and focus actions do not change native content, fixed layout or exports. Describe notification and fullscreen as best-effort/request-and-reported-state boundaries, not guaranteed OS results. Link to the desktop blueprint for detailed interaction and numeric contracts.
6. **Material decisions.** Retain the native semantic project as source of truth and exports as separate outputs, with concise rationale and consequences. Replace the old future implementation/migration consequence with the recorded supported-version boundary; retain other decisions only where they materially clarify the composition. Do not add implementation choices unsupported by the recorded intent.

## Implementation plan

1. Compare the complete original feature blueprint, requirements and supplied decisions against the six groups above. Reorganize into scope and requirements, shared composition, capability realization and decisions following the updated template.
2. Integrate all lasting original model, layout, save/export and trailing editing conditions with the recorded decisions. Correct core-versus-desktop ownership and add toolbar/status realization without duplicating component/container detail. Maintain working relative links and stable traceability with the conditional ID treatment above.
3. Remove lifecycle and current-state/evidence language, WO narratives, benchmarks, placeholders, revision history and obsolete or incompatible recovery, cache and PDF claims. Do not insert the known gaps in the durable target.
4. Compare the final document and scoped diff against every contract, decision, exclusion and traceability rule below. If a material new intent choice, unexplained mismatch or changed ID mapping is necessary, stop and report it to the orchestrator for the user rather than guessing or editing another file.

## Files and systems

- **Create:** none as part of the document edit.
- **Update:** [feature blueprint](../../../blueprints/features/screenplay-writing-loop.md) only.
- **Avoid changing:** all other durable documents, initiative history and existing drafts, source, tests, assets, configuration and Forge processes/templates. The separately authorized PDF source TODO is not part of this WO or its commit.

## Validation and acceptance

- **Document checks:** Compare the entire original and revised blueprint section by section, including trailing refinements, against all six contract groups, the requirements traceability and decisions above. Verify Markdown structure, every retained relative link and any fragment against its actual target, and inspect the exact scoped diff; run `git diff --check` and record failure or unavailability. Run independently applicable project-policy checks supplied by the orchestrator. Application tests and native smoke are not inherently required for this document-only edit; document inspection does not establish runtime behavior.
- **Acceptance:** Given the original blueprint, feature-area template, recorded decisions, and the approved requirements mapping resulting from WO-003, an independent reader can trace all six requirements to clear desktop/core ownership, flows, invariants and failure behavior and verify all six contract groups. The revised feature preserves dialogue split and parenthetical distinctions, fixed-profile layout and replacement-safe cache intent, safe schema-v1 save/reopen, non-ASCII PDF and loss-communicated text, dirty-New consent and native-header ownership. It contains no snapshot promise, incompatible PDF restriction, revision/profile-only cross-session identity claim, WO notes, implementation evidence, progress or history. Cursor ownership is traced to `.7` only after that corrected criterion is present; otherwise it is described without a false identifier. The approved mapping is the execution baseline rather than something still described as unapproved, and no other file changes.
- **Independent validation:** After applicable review, approval and execution, a fresh validator receives this WO, the assigned before/after feature blueprint and the exact dedicated single-parent commit hash. The validator checks that exact commit and document against the acceptance criteria and records the result in the linked validation artifact; document-only validation does not inspect application source or block on known dependent code gaps. Drafting is not execution, approval or validation.

## Durable-document impact

- **Requirements impact:** `verify unchanged` — requirements and IDs are read-only here; orchestrator coordinates separately authorized reconciliation and the proposed duplicate-ID correction.
- **Blueprint impact:** `update` — only the writing-loop feature blueprint; component, container and technical-context documents are read-only.
- **Affected documents/IDs:** Feature blueprint and `REQ-WRITING-LOOP-001`–`006`, applicable acceptance IDs and `NFR-WRITING-001`–`003`; no ID is changed by this WO.
- **Rationale and evidence:** The initiative records the user decisions, existing draft coordination and observed target/code gaps; none authorizes implementation or turns a source observation into target evidence.
- **Authorized documentation changes:** Only the feature blueprint described above; no dependent implementation WO is identified. Orchestrator records any cross-document and document/code lag in the execution handoff when applicable.

## Completion checklist

- [ ] Feature-only edit compared with the complete original, template, all six contract groups, decisions and exclusions.
- [ ] Requirement traceability and proposed ID mapping checked without silent semantic reuse; all links and fragments resolve against actual supplied documents.
- [ ] Scoped diff, Markdown and applicable checks recorded, including unavailable or failed checks.
- [ ] Orchestrator records dispatch/completion identifiers, changed files, evidence and exact dedicated commit hash, or applicable no-change/Git disposition; unrelated source changes excluded.
- [ ] Fresh independent validator receives this WO and exact hash; drafting or implementation alone does not complete validation.
- [ ] Orchestrator coordinates the other document owners, ID correction and recorded gaps without expanding this assignment.

## Handoff

Provide the approved WO, original feature blueprint, updated feature-area template, initiative feature findings, recorded decisions, coordinated ID status and applicable checks. Preserve unrelated work and stop if the scoped edit cannot be isolated safely. Drafting does not authorize execution, review, approval, code changes or a successor WO.

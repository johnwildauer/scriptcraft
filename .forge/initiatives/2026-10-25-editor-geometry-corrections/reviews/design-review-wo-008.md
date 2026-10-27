# Design Review — WO-008 Dialogue Enter Split Policy

## Metadata

- **Review type:** `design`
- **Status:** `blocked`
- **Reviewer:** Fresh-context Forge design reviewer
- **Reviewer session/agent ID:** `architect-design-review-wo-008-2026-10-27`
- **Session exception:** `none`
- **Date:** `2026-10-27`
- **Initiative:** [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:1)
- **Review file:** [`design-review-wo-008.md`](design-review-wo-008.md:1)
- **Reviewed artifact:** [`WO-008-dialogue-enter-split-policy.md`](../work-orders/WO-008-dialogue-enter-split-policy.md:1)

## Review boundary and evidence

This is an independent alignment and handoff-readiness review. I did not
implement code, modify the initiative or work order, run product validation, or
treat implementation evidence as approval.

The comparison boundary is the WO-008 outcome: retain Dialogue→Action only for
Enter at the Dialogue text end, while splitting an interior Dialogue caret into
Dialogue prefix and Dialogue suffix. The review checked:

- [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:139), including the WO-008 amendment and the approved WO-007 boundary.
- [`WO-008-dialogue-enter-split-policy.md`](../work-orders/WO-008-dialogue-enter-split-policy.md:12), its exact oracle, dependency, validation, and durable-impact sections.
- The current [`EditingSession::enter()`](../../../../crates/scriptcraft-core/src/editing.rs:257), [`split_at()`](../../../../crates/scriptcraft-core/src/editing.rs:751), and transactional [`EditingSession::mutate()`](../../../../crates/scriptcraft-core/src/editing.rs:478).
- Requirements [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:38), core blueprint [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:59), feature blueprint [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:78), and [`format-rules.md`](../format-rules.md:32).
- The WO-005/005a predecessor records and the WO-007 → WO-007a → WO-007b → WO-007c chain, including [`validation-WO-007c-v2.md`](../validations/validation-WO-007c-v2.md:1) and [`execution-log.md`](../execution-log.md:111).
- Forge design-review and durable-impact rules in [`03-design-review.md`](../../../process/03-design-review.md:15), [`00-overview.md`](../../../process/00-overview.md:141), and [`04-work-order.md`](../../../process/04-work-order.md:1).

## Findings

### EGC-WO008-DESIGN-B-001 — The declared WO-007 predecessor is not actually validated and its handoff links are inconsistent

- **Severity:** blocker
- **Evidence:** WO-008 declares WO-007 as its predecessor at [`WO-008-dialogue-enter-split-policy.md`](../work-orders/WO-008-dialogue-enter-split-policy.md:8-10) and describes itself as an amendment after the WO-007 chain. However, the recorded chain is not complete: [`validation-WO-007c-v2.md`](../validations/validation-WO-007c-v2.md:5-7) is explicitly `pending-independent-validation`, and its scope guard says WO-007c must remain `in-review` at [`validation-WO-007c-v2.md`](../validations/validation-WO-007c-v2.md:23-25). The execution log likewise records WO-007, WO-007a, WO-007b, and WO-007c as pending fresh validation at [`execution-log.md`](../execution-log.md:124-126). In addition, WO-007 points to [`validation-WO-007-v2.md`](../validations/validation-WO-007-v2.md:1), but that file is absent from the initiative validation directory; the only WO-007-related validation artifacts currently visible are the fix-chain handoffs, including WO-007c.
- **Assessment:** This is not merely stale prose. The WO-008 handoff relies on the completed WO-007 contract for `PageProfile` settings, strict boundary behavior, Parenthetical handling, transactional history, persistence compatibility, and rendered key routing. The required fresh validation has not yet established that the complete predecessor chain is accepted as a stable baseline. Dispatching WO-008 now would create overlapping ownership of the same [`EditingSession`](../../../../crates/scriptcraft-core/src/editing.rs:61) behavior while the predecessor remains formally unresolved, contrary to the per-work-order validation and dependency discipline in [`00-overview.md`](../../../process/00-overview.md:168-178) and the work-order stop rule in [`04-work-order.md`](../../../process/04-work-order.md:28-30).
- **Required action:** Reconcile the predecessor ledger without rewriting history: preserve the implementation records, produce/link the fresh independent validation for the complete WO-007 → WO-007a → WO-007b → WO-007c chain, correct the WO-007 validation link or explicitly record the superseding chain artifact, and make the work-order statuses agree with the resulting validation outcome.
- **Required next state:** Keep WO-008 `draft`; do not dispatch implementation or validation. After the predecessor chain is formally accepted, perform a fresh re-review of this WO-008 handoff before seeking its separate human approval.

### EGC-WO008-DESIGN-W-001 — The end/interior oracle is coherent, but its observable state assertions should be made explicit before implementation

- **Severity:** warning
- **Evidence:** The table at [`WO-008-dialogue-enter-split-policy.md`](../work-orders/WO-008-dialogue-enter-split-policy.md:36-44) correctly distinguishes collapsed `text.len()` from an interior UTF-8 boundary, and explicitly makes an empty Dialogue an end-boundary case. The current implementation at [`EditingSession::enter()`](../../../../crates/scriptcraft-core/src/editing.rs:276-324) currently chooses the role transition without considering whether suffix text remains, so the proposed correction is identifiable. However, the oracle does not state every postcondition that the implementation and validator must observe: the original element retains its stable ID, the new element receives a distinct bounded collision-checked ID, the original selection anchor/focus behavior is not relevant because only collapsed selections are accepted, and the project/selection/history/dirty/revision values for a rejected non-collapsed selection remain unchanged.
- **Assessment:** The missing detail does not prevent understanding the intended feature, and the work order already points to the shared transactional primitives. It is a handoff precision risk, especially because stable identity and transactional no-op behavior are explicit in the in-scope claims at [`WO-008-dialogue-enter-split-policy.md`](../work-orders/WO-008-dialogue-enter-split-policy.md:18-21).
- **Required action:** Add a compact result matrix or validation assertions for prefix text, suffix text, both roles, original/new IDs, selected element/offset, element order, dirty/revision/history, and failed non-collapsed selection. Include repeated interior splits and the `MAX_ELEMENTS`/bounded-ID failure path as unchanged-state cases.
- **Required next state:** Warning disposition in the WO-008 handoff before implementation; no scope expansion is required.

### EGC-WO008-DESIGN-W-002 — Durable updates are correctly classified but their exact authorized contract is not yet named

- **Severity:** warning
- **Evidence:** WO-008 correctly classifies both requirements and blueprint impact as `update` at [`WO-008-dialogue-enter-split-policy.md`](../work-orders/WO-008-dialogue-enter-split-policy.md:65-70). This is required because the current requirement still states unconditional `Dialogue→Action` in [`AC-WRITING-001.4a`](../../../requirements/features/screenplay-writing-loop.md:48), while the core blueprint only states that the session owns explicit Enter policies at [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:67). The work order names the documents but does not identify the exact requirement/blueprint sections or state whether the existing `AC-WRITING-001.4a` line will be amended versus supplemented by a stable WO-008 refinement.
- **Assessment:** The classification is directionally correct and does not broaden scope. The durable contract must distinguish end-boundary and interior Dialogue Enter without accidentally changing Tab, Parenthetical, strict-boundary, or other role behavior.
- **Required action:** Name the exact authorized requirement criterion/refinement and the core/feature blueprint sections to update. Record the final end/interior rule, empty-Dialogue consequence, non-collapsed-selection behavior, and compatibility with the WO-007 matrix. Keep [`format-rules.md`](../format-rules.md:32) unchanged unless implementation evidence demonstrates a formatting contract change.
- **Required next state:** Warning; reconcile the durable-document handoff before implementation and let fresh validation compare the final diff against it.

### EGC-WO008-DESIGN-N-001 — Scope and technical approach are appropriately narrow

The proposed change is aligned with the initiative's writing-surface outcome and is narrower than the completed WO-007 amendment. Reusing [`EditingSession::mutate()`](../../../../crates/scriptcraft-core/src/editing.rs:478), UTF-8 position validation, [`split_at()`](../../../../crates/scriptcraft-core/src/editing.rs:751), and the bounded [`split_id()`](../../../../crates/scriptcraft-core/src/editing.rs:773) preserves the existing core/UI boundary. The exclusions for Tab, Parenthetical policy, geometry, pointer placement, blink, caching, and completed predecessor history are clear at [`WO-008-dialogue-enter-split-policy.md`](../work-orders/WO-008-dialogue-enter-split-policy.md:23-27).

The persistence implication is also bounded: the split changes only canonical semantic elements and stable IDs already represented by native persistence; no new setting, schema field, transient provenance, or layout cache key is proposed. Save/reopen coverage is therefore appropriate as a compatibility check rather than a persistence feature change.

## Handoff assessment

- **Behavior oracle:** **PASS WITH WARNING.** End-boundary versus interior behavior is identifiable, including empty Dialogue and non-collapsed selection behavior. The exact state/identity assertions should be made explicit.
- **UTF-8/selection safety:** **PASS WITH WARNING.** The proposed use of existing position validation and transactional split machinery is sound; Unicode interior boundaries and invalid/non-collapsed selection rollback need explicit matrix assertions.
- **Undo/dirty/revision:** **PASS WITH WARNING.** The shared mutation boundary should provide one undoable mutation for successful splits and no mutation for rejected selections, but the WO should state these results as oracle assertions rather than only list them as tests.
- **Persistence:** **PASS.** No durable schema or transient metadata change is implied. Save/reopen must prove prefix/suffix roles/text and stable IDs survive, without claiming persistence of caret/history.
- **Dependency/scope:** **BLOCKED.** WO-007c remains pending independent validation despite the WO-008 predecessor claim, and the WO-007 validation link is stale/missing.
- **Durable impact:** **PASS WITH WARNING.** `update` is the correct classification for requirements and blueprints; the exact affected sections and final refinement wording are not yet named.
- **Handoff completeness:** **NOT READY.** The implementation seam is clear, but predecessor status reconciliation and the state-result matrix are required before dispatch.

## Verdict and exact next state

- **Verdict:** `BLOCKED — PREDECESSOR VALIDATION LEDGER IS NOT RECONCILED`
- **Outcome:** The WO-008 design is coherent and in scope. The end-boundary/interior Dialogue oracle is understandable and technically compatible with the current session architecture. It cannot pass because the claimed WO-007 prerequisite is not formally complete, its validation links/statuses disagree, and the WO-008 durable/state handoff still leaves observable details to implementer/validator inference.
- **Exact next state:** Keep [`WO-008-dialogue-enter-split-policy.md`](../work-orders/WO-008-dialogue-enter-split-policy.md:5) and the WO-008 amendment section in [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:155) in `draft`. First reconcile and independently validate the complete WO-007 fix chain, correct its status/link ledger, then revise WO-008 with the explicit result/state matrix and exact requirements/blueprint sections. Obtain a fresh WO-008 design re-review and separate human approval before creating an implementer dispatch. Do not implement code, mark WO-008 complete, or infer native typing success from headless/rendered evidence.

## Evidence boundary

This artifact records design alignment only. It claims no code change, Cargo
result, runtime behavior, native smoke, implementation validation, human
approval, or authorization to execute WO-008.

## Fresh reconciliation assessment — 2026-10-27

- **Review mode:** Fresh repository-reading re-review after the prior blocked
  assessment. No product code, requirements, blueprints, initiative plan, or
  work-order artifact was modified. This append records the assessment only;
  the reviewed work order remains the sole implementation handoff.
- **Comparison basis:** Revised [`WO-008-dialogue-enter-split-policy.md`](../work-orders/WO-008-dialogue-enter-split-policy.md:1), this prior blocked review, the complete predecessor chain and fresh [`validation-WO-007bc-v2.md`](../validations/validation-WO-007bc-v2.md:1), current [`EditingSession::enter()`](../../../../crates/scriptcraft-core/src/editing.rs:257), [`split_at()`](../../../../crates/scriptcraft-core/src/editing.rs:751), [`EditingSession::mutate()`](../../../../crates/scriptcraft-core/src/editing.rs:478), persistence, requirements, blueprints, and Forge review/work-order rules.

### Predecessor ledger reconciliation

**PASS.** The former predecessor blocker is resolved. The fresh chain validation
  covers [`WO-007-parenthetical-transition-and-boundary-policy.md`](../work-orders/WO-007-parenthetical-transition-and-boundary-policy.md:1), [`WO-007a-parenthetical-advance-boundary-fix.md`](../work-orders/WO-007a-parenthetical-advance-boundary-fix.md:1), [`WO-007b-rendered-parenthetical-key-route.md`](../work-orders/WO-007b-rendered-parenthetical-key-route.md:1), and [`WO-007c-delimiter-only-parenthetical-delete.md`](../work-orders/WO-007c-delimiter-only-parenthetical-delete.md:1) with `PASS WITH WARNINGS` and no fix work order. Their validation links now point to the chain artifact, and their current metadata says `complete`. The older `in-review` statements in [`execution-log.md`](../execution-log.md:121) are dated implementation/handoff evidence, while the later reconciliation and validation outcome authorize closure; they are not a live dependency contradiction. The absent standalone `validation-WO-007-v2.md` is no longer required because the chain artifact explicitly validates WO-007 as part of the four-work-order chain.

The warnings remain correctly non-blocking: recorded Cargo results were not
independently rerun in the reading-only session, and native typing smoke is not
claimed. This is consistent with [`05-validation.md`](../../../process/05-validation.md:71)
  and the project policy; neither warning prevents the WO-007 baseline from
  being used as WO-008's predecessor.

### Revised WO-008 assessment

- **Scope and dependency: PASS.** The outcome is limited to Dialogue Enter:
  end-boundary Dialogue→Action versus interior Dialogue→Dialogue. The exclusions
  cover Tab, Parenthetical behavior, other role boundaries, geometry, and
  performance. The current source confirms the precise seam: [`EditingSession::enter()`](../../../../crates/scriptcraft-core/src/editing.rs:257) currently sends every collapsed Dialogue caret through [`enter_transition()`](../../../../crates/scriptcraft-core/src/editing.rs:824), while the existing split primitive already preserves the prefix/suffix model and generates a collision-checked ID. No scope drift is visible.
- **End/interior oracle: PASS WITH WARNING.** The revised table correctly makes
  empty Dialogue an end-boundary case, retains Action only at `offset ==
  text.len()`, and requires Dialogue on a valid interior boundary. It also
  preserves the existing invalid-selection behavior for non-collapsed ranges.
  However, the acceptance rows are still summary assertions rather than an
  exact observable state matrix. They do not separately require, for each
  success branch, the original element's unchanged stable ID, the new element's
  distinct bounded collision-free ID, exact prefix/suffix bytes, element order,
  and complete selection anchor/focus state. The no-op row also does not name
  selection unchanged as an assertion, and the `MAX_ELEMENTS`/ID-failure path
  is listed under validation without an explicit unchanged project, selection,
  history, dirty, and revision result.
- **UTF-8 and transactional state: PASS WITH WARNING.** The proposed use of
  checked positions, [`EditingSession::mutate()`](../../../../crates/scriptcraft-core/src/editing.rs:478), and [`split_at()`](../../../../crates/scriptcraft-core/src/editing.rs:751) is compatible with the existing architecture. The handoff now names revision, dirty state, history, undo/redo, and UTF-8, but it still leaves exact before/after values and failed-operation invariants to test interpretation. In particular, it should state that a successful split is exactly one history entry and one revision increment, that undo/redo each restore the exact project and selection state required by the matrix, and that invalid non-collapsed, non-boundary, and capacity failures leave project, selection, history, dirty, and revision unchanged.
- **Persistence/layout: PASS WITH WARNING.** Save/reopen is correctly scoped to
  semantic roles, text, stable IDs, and deterministic layout; caret/history are
  not durable content. The current native persistence model supports this
  assertion. The handoff should nevertheless distinguish the exact saved split
  project from transient session state and state that reopening preserves both
  original/new IDs and byte-exact Unicode prefix/suffix text.
- **Durable handoff: BLOCKED.** The revised handoff names [`REQ-WRITING-LOOP-001`](../../../requirements/features/screenplay-writing-loop.md:38), [`AC-WRITING-001.3`](../../../requirements/features/screenplay-writing-loop.md:46), [`AC-WRITING-001.4`](../../../requirements/features/screenplay-writing-loop.md:47), [`AC-WRITING-001.5`](../../../requirements/features/screenplay-writing-loop.md:50), and the core/feature blueprint files. It does not name the directly conflicting existing criterion [`AC-WRITING-001.4a`](../../../requirements/features/screenplay-writing-loop.md:48), whose durable wording currently says unconditional Dialogue→Action. It also does not provide the exact authorized amendment/refinement text for that criterion, nor identify the exact core and feature blueprint clauses to update beyond broad section links. Because this is a user-facing transition change classified as `Requirements: update` and `Blueprint: update`, the implementer cannot know whether to amend `AC-WRITING-001.4a`, add a stable WO-008 criterion, or do both without inventing durable scope. This remains a handoff blocker under the durable-document rules in [`00-overview.md`](../../../process/00-overview.md:141).

### Required reconciliation before approval

1. Add an explicit end/interior/empty/first/last/repeated-split result matrix to
   [`WO-008-dialogue-enter-split-policy.md`](../work-orders/WO-008-dialogue-enter-split-policy.md:65). Each successful row must assert exact prefix and suffix bytes, roles, order, original ID retention, new distinct bounded ID, selected element/anchor/focus and offset, one history entry, one revision increment, and dirty-state result. Each rejected row must assert unchanged project, selection, history, dirty, and revision, including non-collapsed selection, invalid UTF-8 boundary, and `MAX_ELEMENTS` capacity.
2. Name the durable requirement amendment explicitly: either amend
   [`AC-WRITING-001.4a`](../../../requirements/features/screenplay-writing-loop.md:48) with the end/interior Dialogue rule or add a stable WO-008 criterion and state how it supersedes the unconditional Dialogue clause. Name the exact authorized clauses in [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:63) and [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:78), including the final wording for compatibility with WO-007. Keep [`format-rules.md`](../format-rules.md:32) explicitly unchanged.
3. Keep the predecessor ledger as reconciled; do not rewrite the historical
   WO-007 implementation entries or reopen the passed chain solely to address
   these WO-008 handoff details.

## Fresh verdict and exact next state

- **Verdict:** `BLOCKED — DURABLE HANDOFF AND OBSERVABLE STATE MATRIX REMAIN UNDER-SPECIFIED`
- **Outcome:** The predecessor validation ledger is reconciled, the Dialogue
  end/interior intent is coherent, and the implementation scope is narrow. The
  revised handoff has not fully completed the prior review's required precision
  correction: stable-ID identity/selection and complete transactional state
  assertions remain implicit, and the `Requirements: update` / `Blueprint:
  update` classification lacks an exact authorized amendment for the existing
  unconditional Dialogue criterion.
- **Exact next state:** Keep [`WO-008-dialogue-enter-split-policy.md`](../work-orders/WO-008-dialogue-enter-split-policy.md:5) and the WO-008 initiative amendment in `draft`. Do not dispatch implementation, validation, or human approval for WO-008 yet. Reconcile the three handoff items above, then obtain a fresh WO-008 design re-review. The review must remain separate from implementation and must not modify source or reviewed artifacts.

This fresh assessment claims no implementation, Cargo result, runtime behavior,
native smoke, validation completion, human approval, or execution authorization.

## Final reconciliation assessment — 2026-10-27

- **Review mode:** Fresh-context re-review of the reconciled handoff. No product
  code, requirements, blueprints, initiative plan, or work-order artifact was
  modified. Only this review record was appended.
- **Comparison basis:** Revised [`WO-008-dialogue-enter-split-policy.md`](../work-orders/WO-008-dialogue-enter-split-policy.md:1), the prior blocked findings above, revised [`AC-WRITING-001.4a`](../../../requirements/features/screenplay-writing-loop.md:48), the feature-blueprint [`WO-008 Dialogue Enter facts`](../../../blueprints/features/screenplay-writing-loop.md:137), the reconciled [`WO-007` chain validation](../validations/validation-WO-007bc-v2.md:1), the initiative amendment, applicable Forge process rules, and current [`EditingSession::enter()`](../../../../crates/scriptcraft-core/src/editing.rs:257), [`EditingSession::mutate()`](../../../../crates/scriptcraft-core/src/editing.rs:478), and [`split_at()`](../../../../crates/scriptcraft-core/src/editing.rs:751).

### Reconciliation findings

1. **Exact behavior and state matrix — PASS.** The revised oracle and matrix now
   distinguish empty/end-boundary Dialogue from interior UTF-8-safe Dialogue,
   retain Dialogue→Action only at `offset == text.len()`, preserve the suffix as
   Dialogue for interior splits, and retain the existing non-collapsed-selection
   behavior. The matrix covers first/last/repeated cases, exact prefix/suffix
   preservation, roles, selection offset, stable-ID retention/distinctness,
   bounded IDs, one history entry, one revision increment, dirty-state behavior,
   undo/redo, save/reopen, invalid boundaries, and `MAX_ELEMENTS` failure with
   unchanged state. A collapsed selection's anchor/focus equality is implied by
   the specified collapsed selection result and the existing selection contract;
   this is an implementation-validation assertion, not an unresolved design
   ambiguity.
2. **Transactional and UTF-8 readiness — PASS.** The current source confirms a
   narrow insertion point before the shared transition selection in
   [`EditingSession::enter()`](../../../../crates/scriptcraft-core/src/editing.rs:321).
   The existing [`split_at()`](../../../../crates/scriptcraft-core/src/editing.rs:751)
   preserves byte slices, retains the original element, creates a distinct
   collision-checked ID, and returns the new element at offset zero. The shared
   [`mutate()`](../../../../crates/scriptcraft-core/src/editing.rs:478) clone,
   validation, history, revision, and dirty-state boundary supplies the required
   success and failure semantics. The implementer must prove the matrix with
   focused tests; no implementation behavior is being claimed by this review.
3. **Predecessor ledger — PASS.** The complete [`WO-007 → WO-007a → WO-007b →
   WO-007c` chain](../validations/validation-WO-007bc-v2.md:5) is independently
   recorded as `PASS WITH WARNINGS`, all four work orders are marked `complete`,
   and each handoff points to the chain validation. The recorded Cargo evidence
   and unavailable native smoke are non-blocking warnings under the validation
   policy; no live predecessor dependency contradiction remains.
4. **Requirements amendment — PASS.** The revised work order explicitly
   authorizes amendment of [`AC-WRITING-001.4a`](../../../requirements/features/screenplay-writing-loop.md:48)
   with the exact end-boundary/interior Dialogue rule and states that all other
   listed transitions remain unchanged. The durable requirement now contains that
   rule, eliminating the former conflict with unconditional Dialogue→Action.
5. **Blueprint authorization — PASS.** The handoff identifies the
   [`EditingSession` responsibilities](../../../blueprints/components/screenplay-core.md:29)
   and feature [`Integration Contracts`](../../../blueprints/features/screenplay-writing-loop.md:85)
   as the authorized blueprint update points, explicitly preserves
   [`format-rules.md`](../format-rules.md:32), and the feature blueprint records
   the reconciled Dialogue boundary facts at
   [`WO-008 Dialogue Enter facts`](../../../blueprints/features/screenplay-writing-loop.md:137).
   The proposed update remains within the existing semantic-session and native
   persistence architecture; no schema, layout, or geometry authorization is
   inferred.
6. **Scope and implementation readiness — PASS WITH WARNING.** The scope is
   limited to Dialogue Enter and explicitly excludes Tab, Parenthetical policy,
   other role transitions, geometry, performance, and completed predecessor
   history. The handoff is implementation-ready after the required human design
   approval and fresh implementer-session dispatch. It is not itself permission
   to implement, and this review does not claim Cargo, runtime, desktop, or native
   typing evidence.

### Final verdict and exact next state

- **Verdict:** `PASS WITH WARNINGS — RECONCILED AND IMPLEMENTATION-READY`
- **Warnings:** Implementation and fresh validation must demonstrate the stated
  matrix, including exact byte preservation, stable IDs, collapsed selection
  anchor/focus, no-op state invariants, undo/redo, and save/reopen. Recorded
  predecessor Cargo results remain recorded evidence rather than a rerun in this
  reading session; native typing smoke remains neither performed nor inferred.
- **Exact next state:** WO-008 may proceed to the separate human approval gate.
  After approval, dispatch one fresh implementer session for
  [`WO-008-dialogue-enter-split-policy.md`](../work-orders/WO-008-dialogue-enter-split-policy.md:1),
  then dispatch a separate fresh validation session. Do not treat this design
  review as implementation, validation, human approval, or native typing
  authorization.

This final reconciliation claims no product-code change, implementation result,
Cargo result, runtime behavior, validation completion, or human approval.

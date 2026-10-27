# Design Review — Editor Geometry and Writing Responsiveness Corrections v2

## Review metadata

- **Review type:** `design`
- **Status:** `BLOCKED`
- **Reviewer:** Fresh-context Forge design reviewer
- **Reviewer session/agent ID:** `architect-fresh-context-editor-geometry-corrections-v2-design-review-2026-10-26`
- **Session exception:** `none`
- **Date:** `2026-10-26`
- **Initiative:** [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:1)
- **Review file:** [`design-review-v2.md`](design-review-v2.md:1)
- **Reviewed artifacts:** The v2 plan and all six v2 draft work orders: [`WO-001-role-spacing-and-courier-default.md`](../work-orders/WO-001-role-spacing-and-courier-default.md:1), [`WO-002-continuous-caret-blink.md`](../work-orders/WO-002-continuous-caret-blink.md:1), [`WO-003-nearest-line-pointer-placement.md`](../work-orders/WO-003-nearest-line-pointer-placement.md:1), [`WO-004-role-casing-policy.md`](../work-orders/WO-004-role-casing-policy.md:1), [`WO-005-parenthetical-entry-policy.md`](../work-orders/WO-005-parenthetical-entry-policy.md:1), and [`WO-006-long-document-responsiveness.md`](../work-orders/WO-006-long-document-responsiveness.md:1).
- **Fresh-context statement:** This is an independent alignment review. I did not implement product code, run product validation, or silently change the plan or any work order. The completed v1 artifacts and current source were used as comparison evidence only.

## Comparison boundary and evidence inspected

The comparison boundary is the v2 plan's single observable outcome: a writer can create a long screenplay using normal paragraph entry, role transitions, pointer clicks, and caret navigation while receiving the approved spacing and font behavior, consistent casing and parenthetical entry, and measurable editor responsiveness. The explicit user-approved defaults in [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:61-71), the v2 exclusions, sequential/per-work-order policy, and the shared semantic/layout/source-mapping architecture are also part of the boundary.

Inspected:

- Forge lifecycle, durable-document, dispatch, and design-review rules in [`00-overview.md`](../../../process/00-overview.md:27-205), [`02-initiative.md`](../../../process/02-initiative.md:159-209), and [`03-design-review.md`](../../../process/03-design-review.md:1-72).
- Project policy and approval/branch gates in [`project-manifest.md`](../../../project-manifest.md:46-109).
- The authoritative v2 plan, v2 execution log, and all six v2 work orders.
- The closed predecessor plan [`editor-geometry-corrections-v1.md`](../editor-geometry-corrections-v1.md:1), its prior design review [`design-review.md`](design-review.md:1), predecessor work-order/validation evidence, and [`format-rules.md`](../format-rules.md:1).
- Requirements in [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:36-99), and technical boundaries in [`technical-context.md`](../../../blueprints/technical-context.md:52-87), [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:52-76), and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:35-58).
- Current source in [`PageProfile`](../../../../crates/scriptcraft-core/src/lib.rs:154), [`ScreenplayLayout::build()`](../../../../crates/scriptcraft-core/src/layout.rs:89), [`ScreenplayLayout::hit_test()`](../../../../crates/scriptcraft-core/src/layout.rs:139), [`ScreenplayLayout::place_element()`](../../../../crates/scriptcraft-core/src/layout.rs:241), [`EditingSession::insert()`](../../../../crates/scriptcraft-core/src/editing.rs:138), [`EditingSession::enter()`](../../../../crates/scriptcraft-core/src/editing.rs:231), [`EditingSession::tab()`](../../../../crates/scriptcraft-core/src/editing.rs:244), [`caret_from_pointer()`](../../../../apps/scriptcraft/src/main.rs:238), [`paint_page()`](../../../../apps/scriptcraft/src/main.rs:309), and [`handle_surface_input()`](../../../../apps/scriptcraft/src/main.rs:633).
- Documented WoodCraft research in [`02-layout-pagination-rendering.md`](../../../.analysis/stage2/02-layout-pagination-rendering.md:157-255), [`06-screenplay-mvp-project-structure.md`](../../../.analysis/06-screenplay-mvp-project-structure.md:882-909), and [`08-screenplay-planning-features-and-wordcraft.md`](../../../.analysis/08-screenplay-planning-features-and-wordcraft.md:50-56).

This review does not assess implementation completeness, runtime quality, test results, or whether the desired behavior is industry-standard. The current code confirms that the proposed defects and seams are understandable; it is not treated as evidence that v2 has been implemented.

## Alignment assessment

### Scope coherence

The six work orders are generally coherent with the outcome and do not introduce an unrelated product area. [`WO-001-role-spacing-and-courier-default.md`](../work-orders/WO-001-role-spacing-and-courier-default.md:11-41) addresses shared screenplay geometry and font defaults; [`WO-002-continuous-caret-blink.md`](../work-orders/WO-002-continuous-caret-blink.md:11-33) addresses focused transient caret presentation; [`WO-003-nearest-line-pointer-placement.md`](../work-orders/WO-003-nearest-line-pointer-placement.md:11-33) addresses source-safe pointer mapping; [`WO-004-role-casing-policy.md`](../work-orders/WO-004-role-casing-policy.md:11-29) and [`WO-005-parenthetical-entry-policy.md`](../work-orders/WO-005-parenthetical-entry-policy.md:11-30) address semantic editing affordances; and [`WO-006-long-document-responsiveness.md`](../work-orders/WO-006-long-document-responsiveness.md:11-34) addresses measured performance.

The exclusions consistently preserve the semantic core, fixed print geometry, no bundled font, no second text buffer, and no premature incremental pagination. The v2 plan's WoodCraft boundary is also appropriate: it carries over cache-key, shared-geometry, and nearest-line lessons without importing the broader document model or UI architecture.

### Dependency and order

The proposed sequential order is conservative and understandable. [`WO-001-role-spacing-and-courier-default.md`](../work-orders/WO-001-role-spacing-and-courier-default.md:31-41) establishes geometry and font inputs before pointer and performance work. [`WO-004-role-casing-policy.md`](../work-orders/WO-004-role-casing-policy.md:15-24) and [`WO-005-parenthetical-entry-policy.md`](../work-orders/WO-005-parenthetical-entry-policy.md:15-24) use the existing transactional editing boundary and precede [`WO-006-long-document-responsiveness.md`](../work-orders/WO-006-long-document-responsiveness.md:15-29), so final measurements include their text/layout effects. No impossible dependency or required parallel merge is present. The order is not itself a blocker.

## Findings

### Blockers

#### EGC-V2-DESIGN-B-001 — Initiative is still recorded in `plan`, not `review`

- **Severity:** blocker
- **Evidence:** The authoritative plan records `Phase: plan` at [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:3-6), while Forge requires a formal design review only after the initiative has reached `review` at [`03-design-review.md`](../../../process/03-design-review.md:3-13). The plan's transition evidence records only `draft → plan` at [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:20-22), and the readiness checklist leaves fresh review and human approval open at [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:111-119).
- **Assessment:** The user request supplies a clear readiness signal and the artifacts are sufficient for meaningful alignment comparison, so this is not a scope or comprehension failure. It is an unsatisfied explicit Forge phase gate.
- **Recommendation:** Reconcile the plan's phase to `review` only after recording the `plan → review` transition and confirming the review artifact/work-order set is ready. Verify local branch capability before execution as required by the plan and manifest. Do not dispatch any implementation work order from this blocked ledger.
- **Required next state:** Remain in `plan` until the phase/readiness ledger is reconciled; then enter `review`. After this gate and a separate recorded human approval, the initiative may enter `execute`.

### Warnings

#### EGC-V2-DESIGN-W-001 — Zero-gap default is not an exhaustive role-pair contract

- **Severity:** warning
- **Evidence:** The plan says zero extra gap before new Dialogue and Parenthetical elements at [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:43-50) and [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:63-71). [`WO-001-role-spacing-and-courier-default.md`](../work-orders/WO-001-role-spacing-and-courier-default.md:15-36) asks for a role-pair table, but its named assertions cover Action→Dialogue, Dialogue→Parenthetical, Parenthetical→Dialogue, Character→Dialogue, and representative other pairs; it does not explicitly settle Action→Parenthetical, Character→Parenthetical, Note transitions, same-role boundaries, or whether “above” means the incoming role only.
- **Recommendation:** Before implementation, record a complete role-by-role matrix with `previous_role → next_role → extra gap`, including same-role, Note, page-boundary, empty-element, keep-with-next, and title-page cases. State whether zero applies to every incoming Dialogue/Parenthetical transition or only named pairs. Preserve exact point geometry separately from logical page-capacity accounting.
- **Required next state:** Warning disposition in [`WO-001-role-spacing-and-courier-default.md`](../work-orders/WO-001-role-spacing-and-courier-default.md:43-55) and the v2 plan before dispatch; no requirements change is needed unless this becomes a separately frozen product criterion.

#### EGC-V2-DESIGN-W-002 — Courier default and existing profile compatibility need an explicit migration rule

- **Severity:** warning
- **Evidence:** The current default is `Courier New` in [`PageProfile::default()`](../../../../crates/scriptcraft-core/src/lib.rs:174-200), while accepted persisted families include `Courier`, `Courier New`, and `Courier Prime` in [`validate_profile()`](../../../../crates/scriptcraft-core/src/lib.rs:315-339). [`WO-001-role-spacing-and-courier-default.md`](../work-orders/WO-001-role-spacing-and-courier-default.md:17-24) changes the default but does not state whether existing projects retain their stored family, whether a missing/legacy field receives `Courier`, or whether any profile/schema version changes.
- **Recommendation:** State that the default applies to newly created projects and define behavior for existing serialized profiles, malformed/absent font fields, and explicit supported families. Keep compatibility bounded and avoid silently rewriting existing project settings. Include the exact authorized durable-document update to [`format-rules.md`](../format-rules.md:32-47), which is listed as a changed file but omitted from the work order's durable-impact section.
- **Required next state:** Warning disposition and durable-impact reconciliation in WO-001 before execution.

#### EGC-V2-DESIGN-W-003 — Caret blink acceptance lacks an observable timing/phase contract

- **Severity:** warning
- **Evidence:** [`WO-002-continuous-caret-blink.md`](../work-orders/WO-002-continuous-caret-blink.md:11-24) requires continuous blink, predictable reset, and focused/unfocused repaint behavior, but does not define the blink period, initial phase, reset visibility, or how focus loss, selection, role changes, edits, and repeated frames are observed. The current path uses an egui animation identity keyed by revision/caret and requests repaint after 500 ms in [`paint_page()`](../../../../apps/scriptcraft/src/main.rs:367-385), so “continuous” can be compared but is not yet a precise acceptance oracle.
- **Recommendation:** Define a bounded phase contract: focused collapsed caret, selected range, unfocused surface, focus regain, and caret/revision change; name the expected repaint scheduling interval/tolerance or testable phase transitions without requiring wall-clock visual claims. Preserve animation state outside the model and avoid an unconditional repaint loop.
- **Required next state:** Add the observable blink/reset oracle and affected desktop blueprint evidence before dispatch.

#### EGC-V2-DESIGN-W-004 — Nearest-line pointer scope needs page/title/tie semantics

- **Severity:** warning
- **Evidence:** Current [`ScreenplayLayout::hit_test()`](../../../../crates/scriptcraft-core/src/layout.rs:139-157) returns `None` outside a line rectangle, and [`caret_from_pointer()`](../../../../apps/scriptcraft/src/main.rs:238-254) clamps local coordinates to nonnegative values before calling it. [`WO-003-nearest-line-pointer-placement.md`](../work-orders/WO-003-nearest-line-pointer-placement.md:15-24) correctly requests vertical clamping and deterministic tie order, but does not state what happens on a title page, an empty page, a page with no lines, an exact midpoint tie, or a click in the inter-page gap. “Above/below the document” can mean the selected page's line span or the whole document across pages.
- **Recommendation:** Define the query domain and tie order explicitly: page-local versus document-global, title-page ownership, empty-page behavior, inter-page gaps, vertical distance metric, and horizontal tie behavior. Retain the existing source-byte and UTF-8 boundary guarantees and make the caret↔pointer oracle use the same layout record.
- **Required next state:** Add the boundary matrix to WO-003 before implementation; no scope expansion is required.

#### EGC-V2-DESIGN-W-005 — Casing policy is user-facing and persistence-sensitive but remains conditional

- **Severity:** warning
- **Evidence:** [`WO-004-role-casing-policy.md`](../work-orders/WO-004-role-casing-policy.md:15-24) proposes a persisted/project-profile seam, canonical edit normalization, and behavior during insertion, paste, role assignment, and transitions. It also leaves Unicode behavior to be selected during implementation. The current profile is serialized as part of the native project in [`PageProfile`](../../../../crates/scriptcraft-core/src/lib.rs:154-172), while the requirement currently says canonical text is preserved independently of capitalization at [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:44-50).
- **Recommendation:** Resolve whether enabled-by-default casing is a new frozen requirement or a blueprint-only implementation policy, then classify the requirements impact accordingly rather than leaving it conditional. Define bounded Unicode behavior, expansion cases such as one input scalar becoming multiple output bytes, selection/undo mapping, role-change normalization of pre-existing text, and behavior for already-saved projects. If the seam is persisted, specify compatibility/version handling and update all affected blueprint boundaries, not only the component feature.
- **Required next state:** Warning disposition and explicit requirements/blueprint impact record before WO-004 dispatch.

#### EGC-V2-DESIGN-W-006 — Generated-parenthesis provenance is not specified

- **Severity:** warning
- **Evidence:** [`WO-005-parenthetical-entry-policy.md`](../work-orders/WO-005-parenthetical-entry-policy.md:15-24) requires removing an empty generated `()` while preserving user-authored `()` and non-empty/partial delimiters. The current semantic element stores text and role but no generated-delimiter provenance in [`ScreenplayElement`](../../../../crates/scriptcraft-core/src/lib.rs:105-137), and the existing transactional boundaries in [`EditingSession::enter()`](../../../../crates/scriptcraft-core/src/editing.rs:231-241) and [`EditingSession::tab()`](../../../../crates/scriptcraft-core/src/editing.rs:244-266) do not identify such provenance.
- **Recommendation:** Choose and record the bounded provenance rule: transient session metadata, explicit durable metadata, or a narrowly defined structural predicate. State behavior after save/reopen, undo/redo, paste, partial edits, explicit role assignment, and a user-authored empty pair. The acceptance test must prove that cleanup does not delete user-authored parentheses.
- **Required next state:** Add the provenance and persistence/session boundary to WO-005 and its blueprint impact record before dispatch.

#### EGC-V2-DESIGN-W-007 — Responsiveness has no pass/fail threshold or benchmark oracle

- **Severity:** warning
- **Evidence:** [`WO-006-long-document-responsiveness.md`](../work-orders/WO-006-long-document-responsiveness.md:15-21) requests measurements at 1, 5, 15, 50, and larger page counts and permits caching when measurements justify it, but does not define a target, regression threshold, sampling method, warm-up policy, or a required outcome when no cache is justified. The documented research recommends a 120–180 page benchmark before page-frontier caching at [`02-layout-pagination-rendering.md`](../../../.analysis/stage2/02-layout-pagination-rendering.md:390-400) and explicitly recommends full-layout caching before incremental pagination at [`06-screenplay-mvp-project-structure.md`](../../../.analysis/06-screenplay-mvp-project-structure.md:882-909).
- **Recommendation:** Define the measurement protocol and comparison oracle: fixture construction, warm-up/repetition, timing units, frame/layout/paint/repaint counters, acceptable regression or improvement threshold, and the decision record when caching is not justified. Include a 120–180 page benchmark or document an evidence-based equivalent before considering incremental pagination. Specify that a cache key includes revision plus every geometry/profile input listed by the research, not merely a profile object label.
- **Required next state:** Add the benchmark and pass/fail evidence contract to WO-006; retain incremental pagination as deferred unless the benchmark proves downstream pagination dominant.

#### EGC-V2-DESIGN-W-008 — Work orders do not fully satisfy the durable-impact and requirement-copy handoff

- **Severity:** warning
- **Evidence:** Forge requires each generated work order to contain verbatim applicable requirements, blueprint references, validation steps, and a durable-impact classification with affected documents, rationale/evidence, and authorized documentation changes at [`02-initiative.md`](../../../process/02-initiative.md:176-185) and [`00-overview.md`](../../../process/00-overview.md:141-156). The v2 work orders generally provide IDs and links, but most do not copy the requirement/criterion wording, and the durable sections omit the required rationale/evidence and authorized-document lists. The conditional classifications in [`WO-004-role-casing-policy.md`](../work-orders/WO-004-role-casing-policy.md:37-40), [`WO-005-parenthetical-entry-policy.md`](../work-orders/WO-005-parenthetical-entry-policy.md:37-40), and [`WO-006-long-document-responsiveness.md`](../work-orders/WO-006-long-document-responsiveness.md:43-48) are especially consequential because these changes affect canonical editing behavior, persisted profile/settings seams, or cache ownership.
- **Recommendation:** Normalize all six handoffs before execution: copy applicable requirement and acceptance wording verbatim; name the exact affected requirements, blueprints, and [`format-rules.md`](../format-rules.md:1) where relevant; state the rationale/evidence for `update` or `verify unchanged`; and list authorized documentation changes. Make the classification reflect the final contract rather than leaving all user-facing changes conditional.
- **Required next state:** Revise the six work-order handoffs while retaining their current product scope and order.

#### EGC-V2-DESIGN-W-009 — Native/manual evidence boundary is not consistently stated in each WO

- **Severity:** warning
- **Evidence:** The plan includes native manual smoke when available at [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:36-41), while the project manifest has no configured manual/external hard-stop gate at [`project-manifest.md`](../../../project-manifest.md:88-95). [`WO-002-continuous-caret-blink.md`](../work-orders/WO-002-continuous-caret-blink.md:35-38), [`WO-003-nearest-line-pointer-placement.md`](../work-orders/WO-003-nearest-line-pointer-placement.md:35-39), and [`WO-006-long-document-responsiveness.md`](../work-orders/WO-006-long-document-responsiveness.md:36-41) mention rendered or native checks, but the v2 work orders do not consistently state whether unavailable native evidence is optional, a warning, or a required gate.
- **Recommendation:** Preserve the manifest policy: automated/headless and rendered checks are required where specified; unavailable native smoke is recorded honestly as unavailable unless the project policy is explicitly changed. State the evidence boundary in every applicable WO and do not claim native success from rendered tests.
- **Required next state:** Add the evidence classification to the validation sections; no new project-policy gate is required by this review.

## Notes

- **EGC-V2-DESIGN-N-001 — Scope remains one coherent initiative.** The six symptoms share the fixed profile, semantic editing session, rendered geometry, and long-document outcome. The plan does not broaden into font packaging, rich formatting, a second canonical buffer, or premature pagination architecture.
- **EGC-V2-DESIGN-N-002 — Existing source supports the stated investigation.** The current app rebuilds [`ScreenplayLayout::build()`](../../../../apps/scriptcraft/src/main.rs:409-417) for the rendered surface and unconditionally requests repaint after surface input at [`handle_surface_input()`](../../../../apps/scriptcraft/src/main.rs:633-649), making WO-006's hypotheses testable without presuming that either is the final bottleneck.
- **EGC-V2-DESIGN-N-003 — Predecessor evidence is properly treated as historical.** The closed v1 geometry work established a persisted shared profile and screen/PDF parity, but its universal boundary gap and `Courier New` default are precisely the durable contracts that v2 proposes to refine. The v1 review and validation artifacts do not substitute for v2 approval.
- **EGC-V2-DESIGN-N-004 — Sequential execution is safer than parallel execution.** The work orders touch shared layout, profile, editing, and app paths; the plan's sequential default is consistent with the manifest's preference to avoid unverified parallel ownership.

## Gate assessment

- **Hard-stop design blocker:** [`EGC-V2-DESIGN-B-001`](#egc-v2-design-b-001--initiative-is-still-recorded-in-plan-not-review) — explicit Forge phase gate is unsatisfied.
- **Design alignment:** The intended work is identifiable and aligned; no work order is clearly unrelated or contrary to the approved v2 outcome.
- **Design readiness:** Not ready to advance because the phase ledger is not at `review`, and the warnings above leave several user-approved defaults insufficiently precise for an unambiguous implementation/validation handoff.
- **Execution authorization:** Not granted. This review is not human approval, implementation evidence, or validation evidence.
- **Branch gate:** The v2 plan still requires branch capability verification before implementation dispatch at [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:16-22); the manifest likewise records that capability as unknown at [`project-manifest.md`](../../../project-manifest.md:58-71).
- **Validation boundary:** Retain one fresh validator per work order. Require deterministic core tests for role-pair geometry, profile compatibility, casing, parenthetical provenance, and pointer mapping; rendered tests for caret blink and pointer placement; measured before/after performance evidence for WO-006; and honest treatment of unavailable native smoke.

## Verdict and required next state

- **Verdict:** `BLOCKED — ALIGNED DESIGN, PHASE LEDGER AND HANDOFF PRECISION REQUIRE CORRECTION`
- **Outcome:** The six-work-order scope is coherent and the ordinary order is executable, but the formal design gate cannot pass while the initiative remains in `plan`. The user-approved defaults are directionally clear but not all sufficiently precise for safe implementation and validation, particularly role-pair spacing, parenthetical provenance, casing persistence/Unicode behavior, blink timing, and responsiveness thresholds.
- **Required next state:** Keep the initiative in `plan`; record the `plan → review` transition only after the phase/readiness ledger is reconciled. Before `review → execute`, revise the six work-order handoffs to resolve W-001 through W-009 without silently changing scope, verify branch capability, and obtain separate recorded human design approval for the complete v2 initiative. No implementation dispatch is authorized by this review.
- **Human approval required:** `yes`, at the `review → execute` gate under [`project-manifest.md`](../../../project-manifest.md:88-95) and [`00-overview.md`](../../../process/00-overview.md:90-96).
- **Approval/evidence:** None recorded here. This fresh-context review does not claim user approval, implementation, validation, or closeout.
- **Warning acceptance required:** `no` for ordinary design-review warnings, but their required handoff actions must be completed before implementation because the current work orders are not yet sufficiently precise for reliable acceptance comparison.

## Reconciliation assessment — fresh-context follow-up

### Review boundary and evidence

This follow-up preserves the preceding blocked review and evaluates only the
reconciled v2 artifacts. I re-read the authoritative plan, all six current v2
work orders, the prior findings above, the initiative execution log, the Forge
initiative/design-review rules, the project manifest, the screenplay-writing
loop requirements, the core and desktop blueprints, and the relevant current
layout, editing, profile, export, and desktop source seams. No product code was
implemented, no validation was run, and no phase was advanced by this review.

### Prior finding disposition

| Prior finding | Reconciliation assessment |
|---|---|
| `EGC-V2-DESIGN-B-001` | **Resolved.** The plan now records `Phase: review`, and its phase-transition evidence records `plan → review` on 2026-10-26 after the six work orders and fresh review were prepared. The human approval checkbox remains open, as required. |
| `EGC-V2-DESIGN-W-001` | **Resolved.** The plan now defines the complete body role-pair rule: every incoming Dialogue or Parenthetical has zero extra gap; all other body pairs use `boundary_gap`; same-element lines, title-page exclusion, and logical page-capacity accounting are stated. WO-001 carries the resulting exact oracle. |
| `EGC-V2-DESIGN-W-002` | **Resolved.** The plan and WO-001 distinguish new-project `Courier` defaults from existing stored `Courier`, `Courier New`, and `Courier Prime` values, reject malformed/absent fields as typed load errors, and authorize no schema migration. |
| `EGC-V2-DESIGN-W-003` | **Resolved.** The plan and WO-002 specify the 500 ms focused collapsed-caret phase, controlled repaint scheduling, suppression/reset conditions, and controlled egui-time observation without model mutation or an unconditional repaint loop. |
| `EGC-V2-DESIGN-W-004` | **Resolved with a minor handoff note.** The plan and WO-003 define a document-global body query, vertical-distance selection, earlier-line tie preference, inter-page behavior, title-page ownership, empty-page fallback, document-span clamping, and source-safe in-line hit testing. The plan also records the lower-page-index tie rule; WO-003's shorter boundary table should retain that detail when it is executed. |
| `EGC-V2-DESIGN-W-005` | **Resolved at design level.** The plan and WO-004 define persisted `uppercase_cues`, legacy-default behavior, no rewrite on open, role-limited transactional normalization, Rust Unicode expansion behavior, and caret remapping. The work order correctly identifies the requirement/blueprint update decision as a durable-document consequence rather than silently changing the baseline requirement. |
| `EGC-V2-DESIGN-W-006` | **Resolved.** The plan and WO-005 define transient stable-element provenance, undo/redo inclusion, non-persistence, and the exact cleanup predicate, including save/reopen, pasted/user-authored pairs, partial edits, and non-empty text. |
| `EGC-V2-DESIGN-W-007` | **Resolved.** WO-006 now defines warm-up/sample counts, fixture sizes, edit positions, p50/p95 metrics, instrumentation counters, the 30% improvement and 10% regression oracle, the no-cache outcome, cache-key inputs, and the evidence requirement before incremental pagination. |
| `EGC-V2-DESIGN-W-008` | **Substantially resolved.** Each current WO contains verbatim applicable requirement text, blueprint references, validation steps, a durable-impact section, rationale/evidence, authorized documentation changes, and a final handoff oracle. Two small precision issues remain as warnings: WO-004 and WO-005 retain conditional `requirements: update` language even though their initiative-level defaults are now explicit; and WO-001's authorized-document list does not name the desktop blueprint although the plan lists it among affected blueprints. Neither prevents identifying the intended work. |
| `EGC-V2-DESIGN-W-009` | **Resolved.** All six current WOs state required automated/headless or rendered evidence and separately classify native/manual evidence as optional/unavailable rather than allowing rendered evidence to imply native success. |

### Work-order handoff verification

The six WOs preserve the approved six-symptom scope and dependency order. Each
has a single outcome, explicit in/out-of-scope boundary, applicable requirement
IDs with verbatim text in the reconciled handoff, blueprint references,
implementation guidance, likely files/systems, validation steps, durable-impact
classification, rationale/evidence, authorized documentation changes, and a
native/manual evidence boundary. The plan's spacing, font, blink, pointer,
casing, parenthetical, and performance oracles are represented without adding a
second buffer, font packaging, rich formatting, a new role, or premature
incremental pagination.

The validation links in the six WO metadata headers point to not-yet-created
slugged validation artifacts, while the initiative's standard location uses
`validation-[work-order-id].md`. This is a documentation-link warning, not
implementation or scope drift: no v2 validation is claimed to exist, and the
validation sections themselves define the required evidence boundary. The links
should be normalized before dispatch so the fresh validators have unambiguous
artifact destinations.

### Forge state and readiness

- The authoritative plan is in `review`, with the `plan → review` transition
  recorded. This satisfies the formal design-review dispatch condition.
- The readiness checklist records the fresh review as complete and leaves only
  required human design approval open.
- The initiative remains `active`; no execution dispatch, implementation, or
  validation evidence is claimed.
- Branch capability remains explicitly unverified in both the plan and project
  manifest. It is a pre-dispatch gate, not a reason to move this initiative to
  `execute` during design review.
- The execution log contains historical predecessor/v1 activity, but no v2
  implementation dispatch. That history does not alter the v2 scope or gate.

## Reconciliation verdict

- **Verdict:** `PASS WITH WARNINGS — RECONCILED DESIGN IS ALIGNED AND READY FOR HUMAN APPROVAL`
- **Scope decision:** No scope drift found. All six work orders remain subordinate to the single observable writing-loop outcome and the explicit v2 exclusions.
- **Resolved-gate decision:** The prior phase blocker is resolved; the plan is correctly in `review`, not `plan` or `execute`.
- **Remaining warnings:** Normalize the six planned validation-artifact links before dispatch; carry the lower-page-index tie detail into WO-003's executable boundary table; make WO-004/WO-005 requirement-update classification explicit at the durable-document decision point; and include the desktop blueprint in WO-001's authorized-document list if it is to be updated under the plan.
- **Execution authorization:** Not granted by this review. A passing design review is not human approval, implementation evidence, or validation evidence.
- **Exact next state:** Remain in `review` with the initiative `active`. Obtain separate recorded human design approval, verify local branch capability, and normalize the noted handoff documentation before any dispatch. Only after those conditions are recorded may the orchestrator evaluate the `review → execute` gate. Do not dispatch a work order or move the initiative to `execute` from this review.

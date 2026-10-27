# Review: Design — Title-Bar Transition and Elements Alignment v4

## Review metadata

- **Review type:** `design`
- **Status:** `in-review`
- **Reviewer:** Fresh-context Forge design reviewer
- **Reviewer session/agent ID:** Not exposed by the review interface; no session identifier is inferred.
- **Session exception:** `none`
- **Date:** `2026-10-27`
- **Initiative:** [`titlebar-transition-and-elements-alignment-v4.md`](../titlebar-transition-and-elements-alignment-v4.md:1)
- **Review file:** [`design-review-v4.md`](design-review-v4.md:1)
- **Reviewed artifact:** v4 plan and sole work order [`V4-WO-001-vanilla-native-titlebar-comparison.md`](../work-orders/V4-WO-001-vanilla-native-titlebar-comparison.md:1)

## Scope of review

The comparison boundary is the single v4 outcome: establish a temporary, reversible ordinary native macOS titlebar baseline, prove the rebuilt app actually displays native chrome before interpreting any animation, then record the user's separate maximize and restore observations against the previously user-confirmed custom hidden-titlebar swell/shrink/snap result. This is an evidence-gathering experiment, not a fix claim, native-animation-control claim, or permanent native-titlebar product decision.

Inspected:

- [`.forge/_index.md`](../../../_index.md:1), [`00-overview.md`](../../../process/00-overview.md:1), [`02-initiative.md`](../../../process/02-initiative.md:1), and [`03-design-review.md`](../../../process/03-design-review.md:1)
- [`project-manifest.md`](../../../project-manifest.md:1)
- [`titlebar-transition-and-elements-alignment-v4.md`](../titlebar-transition-and-elements-alignment-v4.md:1) and its sole work order, [`V4-WO-001-vanilla-native-titlebar-comparison.md`](../work-orders/V4-WO-001-vanilla-native-titlebar-comparison.md:1)
- The relevant v3 failure and pending-gate evidence in [`V3-WO-002a`](../work-orders/V3-WO-002a-native-titlebar-scaling-diagnostic.md:1), [`V3-WO-002c`](../work-orders/V3-WO-002c-native-titlebar-visibility-handoff.md:1), [`V3-WO-002d`](../work-orders/V3-WO-002d-native-titlebar-direct-configuration.md:1), and [`execution-log.md`](../execution-log.md:337)
- The passing toolbar/black-bar preservation evidence in [`validation-V3-WO-001b.md`](../validations/validation-V3-WO-001b.md:1)
- [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:90) and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:105)
- Current [`main.rs`](../../../../apps/scriptcraft/src/main.rs:47), including mode ownership, startup options/handoff, and conditional title-strip rendering; [`apps/scriptcraft/Cargo.toml`](../../../../apps/scriptcraft/Cargo.toml:8); and [`Cargo.lock`](../../../../Cargo.lock:874)
- Resolved API findings recorded by the prior v3 direct-configuration audit in [`V3-WO-002d`](../work-orders/V3-WO-002d-native-titlebar-direct-configuration.md:84)

This is a design-alignment/readiness review, not source validation, a live macOS smoke test, or human implementation approval. No plan, work order, source, v3/history, requirement, blueprint, validation, or execution log was modified. The review is limited to v4 and does not reopen v1-v3 scope.

## Findings

### Blockers

- **None.** The intended work is identifiable, directly related to the requested native-versus-custom comparison, bounded to one work order, and not contrary to an explicit user decision or project-policy gate.

### Warnings

- **V4-DESIGN-W-001 — Reconfirm resolved native-window API behavior at implementation time.** **Severity:** warning; non-blocking for design alignment. **Evidence:** The current manifest resolves eframe `0.29.1` and winit `0.30.13` ([`Cargo.lock`](../../../../Cargo.lock:874), [`Cargo.lock`](../../../../Cargo.lock:3818)). The recorded v3 direct-configuration audit says resolved egui-winit `0.29.1` ignores the requested `titlebar_buttons_shown` field, so a requested option and unit test are not proof of visible native buttons ([`V3-WO-002d`](../work-orders/V3-WO-002d-native-titlebar-direct-configuration.md:87-93)). The local Cargo registry source path cited by that audit was unavailable to this review session, so the underlying dependency source was not independently reread here. The v4 work order correctly requires a fresh resolved-default/API audit before options are changed and uses actual rebuilt-window observation as the visual oracle ([`V4-WO-001`](../work-orders/V4-WO-001-vanilla-native-titlebar-comparison.md:22-28,58-61,73-77)). **Recommendation:** Preserve that audit as a pre-change implementation step; record the actual resolved versions and relevant default/forwarding behavior. Do not interpret a configuration field, a test, or startup diagnostic as proof of native chrome. **Required next state:** `review`; informational for the design outcome, but implementation must satisfy the specified audit and visual gate.

### Notes

- **V4-DESIGN-N-001 — The last v3d visual outcome remains unknown, not passed or failed.** **Severity:** note. **Evidence:** The latest v3d record says automated checks passed but the user visual gate was still outstanding ([`execution-log.md`](../execution-log.md:411), [`V3-WO-002d`](../work-orders/V3-WO-002d-native-titlebar-direct-configuration.md:93)). Earlier user reports of hidden/transparent chrome belong to the preceding v3 diagnostic attempts, including v3c ([`execution-log.md`](../execution-log.md:400-409)); they must not be substituted for an observation of the v3d rebuilt binary. The v4 plan states this distinction and leaves the latest visible state unknown ([`titlebar-transition-and-elements-alignment-v4.md`](../titlebar-transition-and-elements-alignment-v4.md:26-54)). **Recommendation:** Preserve this evidence boundary; do not claim the v3d native prerequisite or any native animation result.
- **V4-DESIGN-N-002 — Native-default meaning is constrained safely.** **Severity:** note. **Evidence:** The plan and work order call for the ordinary decorated platform path, require auditing resolved defaults before changing options, preserve unrelated startup settings, avoid another diagnostic override layer, and use the ordinary product title ([`titlebar-transition-and-elements-alignment-v4.md`](../titlebar-transition-and-elements-alignment-v4.md:62-69), [`V4-WO-001`](../work-orders/V4-WO-001-vanilla-native-titlebar-comparison.md:22-25,58-60)). They do not equate an API request with a guaranteed macOS appearance; the prerequisite requires the user to confirm real, visible, non-transparent native chrome and no duplicate custom strip before animation evidence is accepted ([`V4-WO-001`](../work-orders/V4-WO-001-vanilla-native-titlebar-comparison.md:73-77)). **Recommendation:** Retain this distinction in implementation and reporting; “native default” means the audited ordinary decorated path under test, not proof that macOS displayed it.
- **V4-DESIGN-N-003 — The comparison is reversible and preserves the prior custom path.** **Severity:** note. **Evidence:** Current source has a selected compile-time presentation mode, custom-only title-strip/maximize/policy ownership, and one startup handoff ([`main.rs`](../../../../apps/scriptcraft/src/main.rs:47-98), [`main.rs`](../../../../apps/scriptcraft/src/main.rs:2419-2457), [`main.rs`](../../../../apps/scriptcraft/src/main.rs:2563-2642)). The work order limits the change to the experiment's selected startup path, retains the custom hidden-titlebar route for rollback, and excludes broad removal/refactoring of transition helpers ([`V4-WO-001`](../work-orders/V4-WO-001-vanilla-native-titlebar-comparison.md:24-35,58-60)). **Recommendation:** Keep rollback availability source-level and tested; do not add a selector or remove helpers as part of this experiment.
- **V4-DESIGN-N-004 — The visual gates are ordered, observable, and not replaceable by automation.** **Severity:** note. **Evidence:** The native-chrome prerequisite explicitly precedes animation observation; if it fails, animation is not comparison evidence. Maximize and restore are separately observed, and the report distinguishes same, absent in both directions, and different/partial outcomes ([`titlebar-transition-and-elements-alignment-v4.md`](../titlebar-transition-and-elements-alignment-v4.md:70-79), [`V4-WO-001`](../work-orders/V4-WO-001-vanilla-native-titlebar-comparison.md:73-77)). The clean build identity and user observations must be recorded before the work order is handed to a fresh validator ([`V4-WO-001`](../work-orders/V4-WO-001-vanilla-native-titlebar-comparison.md:58-61,87-100)). No native animation result is claimed in the plan or this review. **Recommendation:** The orchestrator must ensure the user's prerequisite and both direction-specific observations are written into the work order and execution log before dispatching fresh validation; missing visual evidence means the WO is not ready for validation/completion.
- **V4-DESIGN-N-005 — Preservation and durable-document classifications are appropriate to the temporary experiment.** **Severity:** note. **Evidence:** The work order carries forward the toolbar allocation and `128.0` Elements width, black-bar/clear-color/restored-padding result, and Elements/popup/cursor/input/IME/accessibility/semantic/page behavior as explicit preservation scope ([`V4-WO-001`](../work-orders/V4-WO-001-vanilla-native-titlebar-comparison.md:26,46-50,68)). The passing [`validation-V3-WO-001b.md`](../validations/validation-V3-WO-001b.md:48-60,63-80) supplies the relevant preserved baseline. Requirements `verify unchanged` is accurate because the temporary path does not redefine the title-strip product requirement; blueprint `verify unchanged` is accurate because no persistent architecture/product decision is authorized ([`V4-WO-001`](../work-orders/V4-WO-001-vanilla-native-titlebar-comparison.md:79-85); [`00-overview.md`](../../../process/00-overview.md:141-156)). **Recommendation:** Reconcile these classifications against the eventual diff; if the native path becomes durable or a technical contract changes, require separately authorized durable-document updates rather than treating this experiment as the product decision.

## Questions for follow-up

- None required to compare the plan and work order with the stated outcome. The resolved API/default audit and user visual observations are specified execution evidence, not unanswered planning questions.

## Traceability

- **Requirements checked:** [`REQ-WRITING-LOOP-005`](../../../requirements/features/screenplay-writing-loop.md:90), [`AC-WRITING-005.5`](../../../requirements/features/screenplay-writing-loop.md:115), [`AC-WRITING-005.6`](../../../requirements/features/screenplay-writing-loop.md:131), and [`AC-WRITING-005.7`](../../../requirements/features/screenplay-writing-loop.md:137)
- **Blueprints checked:** [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:105-134)
- **Work orders checked:** sole v4 order [`V4-WO-001-vanilla-native-titlebar-comparison.md`](../work-orders/V4-WO-001-vanilla-native-titlebar-comparison.md:1); v3 evidence orders [`V3-WO-002a`](../work-orders/V3-WO-002a-native-titlebar-scaling-diagnostic.md:1), [`V3-WO-002c`](../work-orders/V3-WO-002c-native-titlebar-visibility-handoff.md:1), and [`V3-WO-002d`](../work-orders/V3-WO-002d-native-titlebar-direct-configuration.md:1)
- **Explicit user decisions checked:** Proceed with work-order preparation/execution subject to visual-feedback gates; no presumption of the last v3d visual result; native-versus-custom comparison is evidence gathering only; no native animation outcome or permanent native-titlebar decision is authorized by this experiment.
- **Comparison evidence sufficient:** `yes` for design alignment. The known custom-path swell/shrink/snap observation is a valid comparator; v3d native visibility/animation remains unobserved. The future native gate is intentionally not claimed as already satisfied.
- **Explicit project-policy gates checked:** Separate passing design review and human approval at `review -> execute`; current base and local branch capability/required initiative branch before implementation dispatch; one fresh implementer and a separate fresh validator; sequential per-WO validation; manifest Cargo/diff checks. See [`project-manifest.md`](../../../project-manifest.md:61-71,73-95,97-102) and [`00-overview.md`](../../../process/00-overview.md:90-114,168-184).

## Decision

- **Outcome:** `PASS WITH WARNINGS`
- **Severity:** No design blocker. The warning is a bounded API-evidence limitation for this review session; the work order already requires the implementation session to resolve it before changing options.
- **Recommendation:** Retain the v4 plan and sole work order as aligned. During execution, audit the resolved defaults/API first, preserve the custom rollback path and listed contracts, and record the clean-build identity plus the native-chrome prerequisite and separate maximize/restore observations before fresh validation. No native animation result or permanent product choice may be inferred.
- **Required next state:** `review` — design review passes with a non-blocking warning. Keep the initiative gated; before dispatching implementation, obtain separate explicit human design approval and record current base/HEAD and branch-capability evidence, creating the required initiative branch if policy applies. The current review does not authorize or perform that transition.
- **Human approval required:** `yes`
- **Approval/evidence:** None recorded. The user's authorization to prepare/proceed conditionally does not substitute for the separate post-review human design approval required by [`project-manifest.md`](../../../project-manifest.md:88-95). No approval is granted or inferred here.
- **Warning acceptance required:** `no`
- **Evidence boundary:** This review assesses the v4 design only. It does not claim implementation, clean-check results, successful native chrome visibility, any native animation result, validation, or closeout.

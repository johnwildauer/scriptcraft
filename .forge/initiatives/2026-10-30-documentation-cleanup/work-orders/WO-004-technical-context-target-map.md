# Work Order: WO-004 — Technical Context Target Map

## Metadata

- **Status:** `draft`
- **Work-order kind:** `document-only`
- **Assignee:** To be assigned by the orchestrator after applicable gates.
- **Order/dependencies:** No hard predecessor WO; coordinate terminology and recorded decisions with WO-001–003 and the separately assigned blueprint owners. Independent design review and required human design approval precede execution. No dependent implementation WO is identified; the orchestrator owns the recorded text-export gap and any future disposition.
- **Initiative:** [`../documentation-cleanup-v1.md`](../documentation-cleanup-v1.md)
- **Generated filename:** `WO-004-technical-context-target-map.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-004.md`](../validations/validation-WO-004.md) — future artifact.
- **Fixes:** none
- **Implementer session/agent ID:** pending dispatch
- **Dedicated work-order commit:** pending; exact full single-parent hash required when Git is usable.
- **Documentation-first relationship:** none; no dependent implementation WO is identified. A known apparent desktop text-export integration gap is an initiative-level follow-up, not assigned code work.
- **Commit description convention:** `docs-first (pre-implementation): no dependent implementation WOs — technical context target map`; the conventional label does not assert a code dependency.
- **Session exception:** none

## Summary

Populate the unfilled technical-context scaffold with a concise, navigable target map of ScriptCraft's desktop and reusable core architecture. Describe intended structure in established-state voice, with no work-order observations, progress, evidence claims or history in the durable blueprint. The target guides future work and is not proof of implementation.

## In scope

- Edit only [`technical-context.md`](../../../blueprints/technical-context.md); replace scaffold instructions and placeholders with the existing sections populated at a high, navigable level.
- Describe the workspace, principal paths, runtime and delivery boundary, technology/dependency direction, cross-cutting contracts, verification approach and durable constraints as specified below.
- Apply the recorded decisions: no recovery snapshot promise or invented replacement; retain atomic/failure-safe native saving and deliberately lossy text export with loss communication; retain the native macOS header rather than the retired app-owned title strip.

## Out of scope

- Editing other blueprints, requirements, the manifest, source, tests, configuration, process/templates, historical artifacts or execution records as part of this document edit. Implementing features, deleting snapshot code, creating an implementation successor or conducting review/approval is not authorized.
- Inventing a service, cloud architecture, authentication, secrets store, persistent preferences, distribution method, supported OS matrix, text-export UI design or additional recovery capability. Do not infer native OS notification, fullscreen or visual behavior from non-native checks.
- Treating current code as sole authority to change intended behavior, or repeating source evidence, implementation gaps, lifecycle status, owners, dates, work-order history or progress in the target document. Do not expand into detailed component contracts or an exhaustive repository inventory.

## Requirements

- [`product-overview.md`](../../../requirements/product-overview.md) and [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md) provide the intended single-writer desktop writing loop, headless-capable core, fixed layout, semantic native save safety, PDF and deliberately lossy text with communicated loss. The [initiative decisions](../documentation-cleanup-v1.md) supersede their unreconciled snapshot and app-owned title-strip wording for this assignment; preserve all other relevant intent.
- The text-export intent is not evidence of a working end-to-end desktop route. Do not discard that intent or imply an implementation has been validated; the initiative owns the actual/target gap.

## Blueprints and evidence

- [`technical-context.md`](../../../blueprints/technical-context.md) is both the sole target and the applicable section scaffold. Forge's [artifact boundary](../../../_index.md) specifies a target map in established-state voice, not a current-state inventory.
- Link the independently runnable [desktop container](../../../blueprints/containers/scriptcraft-desktop.md), reusable [screenplay core](../../../blueprints/components/screenplay-core.md), and [writing-loop composition](../../../blueprints/features/screenplay-writing-loop.md). These are navigation/context, not authority to copy their outdated recovery, titlebar, history or implementation-evidence language.
- Map signposts supported by the [workspace manifest](../../../../Cargo.toml:1), [desktop manifest](../../../../apps/scriptcraft/Cargo.toml:8), [desktop entry](../../../../apps/scriptcraft/src/main.rs:8), [core manifest](../../../../crates/scriptcraft-core/Cargo.toml:1), [core modules](../../../../crates/scriptcraft-core/src/lib.rs:4), [export adapters](../../../../crates/scriptcraft-core/src/exports.rs:46), [core tests](../../../../crates/scriptcraft-core/tests/), and [font assets](../../../../fonts/CourierPrime/). Evidence belongs in this WO/initiative, not in the durable map.

## Implementation plan

1. Under **Project map**, use a short table for the Cargo workspace manifest; desktop entry and UI/platform modules; reusable core entry and editing, layout, persistence and export modules; core test area; bundled fonts; and Forge requirements/blueprints. Use working relative links to the significant locations. Exclude generated/dependency/build output and avoid an exhaustive file listing.
2. Under **Runtime and delivery** and **Technology and boundaries**, describe a local Rust desktop executable built/run through the Cargo workspace, the eframe/egui and native-file-dialog desktop boundary, and its dependency on a headless-capable core. The core is a reusable library, not another independently runnable container. Link the desktop container and the core/feature blueprints. Do not prescribe a distribution mechanism, permanent framework version, broader OS support, or an unverified deployment pipeline.
3. Describe the principal relationship: desktop input delegates semantic operations to the core's session and project; shared fixed-profile layout supplies desktop presentation and PDF; versioned native project persistence uses atomic replacement; PDF and deliberately lossy text are noncanonical outputs, with text loss communicated. Preserve the target text-export intent without inventing a desktop route. Keep transient view state separate from canonical project content and viewport zoom separate from print geometry.
4. Under **Cross-cutting design** and **Architectural constraints**, summarize transactional/failure-safe state and saves, privacy-conscious errors/diagnostics, bounded semantic data, deterministic layout and headless verification. Mention native macOS header ownership only at an appropriate boundary; do not add behavior to the OS-owned header. Identify relevant automated core/workspace checks and desktop/manual checks where applicable without reporting a run. Omit irrelevant identity/secrets sections or briefly say they are not part of the described local product; do not assert an unverified configuration mechanism or deployment policy.
5. Check the map against the explicit user decisions and the acceptance criteria. If a material unsupported architecture or intent choice is necessary, or an additional material mismatch is discovered, stop and report it to the orchestrator for user decision rather than guessing or editing another file.

## Files and systems

- **Create:** none as part of the document edit.
- **Update:** [`technical-context.md`](../../../blueprints/technical-context.md) only.
- **Avoid changing:** all other durable documents, historical artifacts, application/core source, tests, assets, configuration and Forge process/templates.

## Validation and E2E acceptance tests

- **Automated checks:** Run `git diff --check` for the scoped edit and report results or unavailability; run any independently applicable project-policy gates supplied by the orchestrator. Application build/test execution is not inherently required for this document-only change. Do not claim a runtime or native smoke pass from a documentary check.
- **Manual checks:** Inspect the exact scoped diff and Markdown structure; verify each map path and relative document link resolves, the desktop is the only identified runnable container, each listed section conveys its assigned purpose, and no placeholder or template instruction remains. Compare the document against every decision, exclusion and map/contract requirement above; report failed or unavailable checks.
- **Acceptance:** Given the existing scaffold and the recorded decisions, when the completed target document is compared with this WO, then an independent reader can navigate the workspace and linked container/core/feature documents, understand ownership and the input/layout/persistence/export flows, and identify verification and durable constraints. The document uses established-state target voice, contains no snapshots promise or app-owned title-strip requirement, retains safe native save and lossy text with loss communication, and does not claim implementation evidence, history, progress, invented infrastructure or unverified native behavior. No file outside the sole target is changed by the WO.
- **Independent validation:** After applicable review, approval and execution, a fresh validator receives this WO and its exact dedicated single-parent commit hash, reviews that exact commit and the assigned document against these criteria, and records the result in the linked validation artifact. Document-only validation does not inspect application source or block on unimplemented dependent code; drafting implies no validation pass.

## Durable-document impact

- **Requirements impact:** `verify unchanged` — requirements are read-only; the recorded user decisions must be reflected without editing the requirements here.
- **Blueprint impact:** `update` — only the technical-context target map is authorized; other blueprint reconciliation remains separately owned.
- **Affected documents/IDs:** Technical context; product overview and writing-loop requirements/blueprints are coordination references, with no ID changes.
- **Rationale and evidence:** The unfilled map cannot currently navigate the established desktop/core boundary. The initiative records the supporting evidence, user decisions, unreconciled documents and apparent end-user text-export gap.
- **Authorized documentation changes:** The single target described above. No dependent implementation WO is identified; the orchestrator records any document/code lag in the initiative execution handoff when applicable.

## Completion checklist

- [ ] Target-only map populated and checked against all paths, flows, decisions, exclusions and acceptance criteria.
- [ ] Markdown, links, scoped diff and applicable policy checks recorded; failed or unavailable checks reported.
- [ ] Orchestrator records dispatch/completion identifiers, changed files, evidence and exact dedicated commit hash, or the applicable no-change/Git disposition.
- [ ] Fresh independent validator receives this WO and exact hash; drafting or implementation alone does not complete validation.
- [ ] Orchestrator coordinates other document owners and the recorded text-export gap without expanding this assignment.

## Handoff

Provide the approved WO, original technical-context scaffold, initiative technical-context finding and recorded user decisions, and applicable checks. Preserve unrelated work and stop if the target change cannot be isolated safely. Drafting does not authorize execution, review or approval.

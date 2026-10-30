# Work Order: WO-001 — Project Manifest Cleanup

## Metadata

- **Status:** `executed; narrow scope-alignment outcome recorded`
- **Work-order kind:** `document-only`
- **Assignee:** User-directed execution; individual implementer/session not recorded.
- **Order/dependencies:** No predecessor or dependent implementation WO. At drafting, independent design review and required human approval were the default pre-execution gates. The later user direction explicitly authorized execution without a separate design-review agent; no formal design review or approval is recorded.
- **Initiative:** [`../documentation-cleanup-v1.md`](../documentation-cleanup-v1.md)
- **Generated filename:** `WO-001-project-manifest-cleanup.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-001.md`](../validations/validation-WO-001.md) — narrow changed-file scope alignment; no implementation-scope blocker identified; not full acceptance validation.
- **Fixes:** none
- **Implementer session/agent ID:** Not recorded.
- **Dedicated work-order commit:** `fd89a0c6c03a082d9f1163147e86e2d35ca7e0aa`.
- **Documentation-first relationship:** none; manifest cleanup is not a target for a dependent code change.
- **Commit description convention:** `docs-first (pre-implementation): no dependent implementation WOs — project manifest cleanup`; the conventional label does not assert a code dependency.
- **Session exception:** none

## Summary

Make the project manifest a clear, trustworthy project-policy and baseline reference without WO-specific commentary or a revision-history table. Preserve substantive policy, decisions, uncertainty, and traceable initialization evidence; do not change product intent or infer that unverified checks passed.

## In scope

- Edit only [`project-manifest.md`](../../../project-manifest.md): clarify formatting, refresh or qualify stale initialization-era baseline facts, and remove WO-specific narrative and revision-history entries.
- Preserve initialization completion, project intake and non-goals, guided working agreement, technical constraints, project policies and safeguards, roadmap revision rule, still-open decisions and owners, and meaningful initialization/review evidence.
- Keep a concise dated baseline and links to existing evidence; retain current commands as project policy without representing old runs as current results.

## Out of scope

- Product or architecture changes, source/test edits, requirements or blueprint edits, changes to process or historical artifacts, broad repository audit, and new roadmap or project-policy decisions.
- Inventing current branch, remote, PR, operating-system, export, framework approval, check results, or resolved decisions; do not weaken a policy in the name of cleanup.

## Requirements and blueprints

- No product requirements or architecture contracts are being changed. The manifest's own durable responsibilities and policy govern this assignment; see [`_index.md`](../../../_index.md) and [`01-init-existing.md`](../../../process/01-init-existing.md).

## Implementation plan

1. Compare the manifest to the plan's manifest-specific findings and supplied evidence. Preserve substantive decision and policy content in the appropriate current sections before removing historical prose; preserve uncertainty, owner and next action where still applicable. If an apparent policy change or materially unsupported conclusion is necessary, stop and report it for user decision.
2. Reconcile the baseline: distinguish dated initialization facts from current evidence. The scaffold records successful creation of the initiative branch from `master`; [`Cargo.toml`](../../../../Cargo.toml) identifies the workspace members and [`apps/scriptcraft/Cargo.toml`](../../../../apps/scriptcraft/Cargo.toml) identifies current dependencies. Replace obsolete no-source/no-commit, unknown-branch, dependency-free, no-framework, foundation-only output, and old test-count claims with evidence-supported dated observations or clearly marked unknowns. Do not infer new run results or convert a present dependency into an approved future product mandate. Recheck time-sensitive facts when feasible; otherwise qualify or mark them unknown.
3. Keep the existing command names and safety/review/branch rules, but remove stale pass/count/output assertions or attribute them to dated historical evidence; do not claim fresh verification. Keep setup and manual-gate policy accurate without weakening it; if their present applicability cannot be established, label it unknown and retain the requirement to discover required checks rather than assuming none.
4. Recast the first-initiative handoff as historical and link the actual [`screenplay MVP plan`](../../2026-10-25-screenplay-mvp/2026-10-25-screenplay-mvp-v1.md) and [`foundation validation`](../../2026-10-25-screenplay-mvp/validations/validation-WO-001.md). Preserve unresolved export/remote/PR and review-warning follow-ups unless evidence supports a narrower factual status; avoid announcing a new roadmap. Retain links to the existing [`requirements review`](../../../requirements/requirements-review.md) and [`blueprint review`](../../../blueprints/blueprint-review.md), and do not claim warnings are resolved without evidence.
5. Remove the revision-history table and WO-specific implementation narrative, retaining necessary project-level decisions in current sections and concise links to existing evidence. Verify Git history is available before relying on it; if unique material cannot otherwise be preserved, stop and report the gap. Do not alter initiative history or manufacture a substitute revision ledger. Update revision metadata accurately without introducing a new history section.

## Files and systems

- **Update:** [`project-manifest.md`](../../../project-manifest.md) only.
- **Create:** none as part of the document edit.
- **Avoid changing:** other durable documents, initiative histories, application code, tests, configuration, and Forge process/templates.

## Validation and acceptance

- **Document checks:** Review the complete manifest diff against the original section by section; verify that each policy, durable decision, unresolved owner/next action, and initialization review link is retained or accurately represented. Check Markdown headings, lists, tables and relative links, and run `git diff --check`. Report any unavailable check rather than claiming it passed. Application build/test execution is not required for a manifest-only edit unless an applicable project gate independently requires it.
- **Acceptance:** Given the existing manifest and linked evidence, when the scoped edit is reviewed, then no obsolete initialization snapshot reads as an unqualified current fact; commands are not falsely reported as freshly passing; no revision table or WO-specific notes remain; baseline, project policy, unresolved decisions, historical handoff and initialization-review traceability remain clear and intact; and no other durable document or product behavior has changed.
- **Independent validation:** After applicable approval and execution, a fresh validator receives this WO and its exact dedicated single-parent commit hash, checks the exact commit/diff and these criteria, and records the result in the validation artifact. Do not validate dependent code or infer a pass from drafting.

## Durable-document impact

- **Project manifest:** `update` — only authorized durable target.
- **Requirements impact:** `not applicable` — no user-facing behavior or product intent is changed.
- **Blueprint impact:** `not applicable` — no architecture or technical contract is changed.
- **Documentation/code lag:** none intended; no dependent implementation WO.

## Completion checklist

- [ ] Scoped manifest edit and preservation checks completed; no unsupported fact or silent policy change.
- [ ] Document checks and applicable gates recorded, including unavailable or failed checks.
- [x] Orchestrator records changed target, execution evidence and exact dedicated WO commit hash; individual session ID was not recorded.
- [x] Fresh independent narrow scope validator received this WO and exact hash; see linked validation. Broader acceptance remains unverified.

## Handoff

The orchestrator supplies the approved WO, current manifest, plan's manifest-specific finding, relevant existing evidence, and required checks. Keep unrelated work isolated; if pre-existing changes cannot be separated safely, stop. User approval to draft is not approval to execute.

## Execution record — 2026-10-30

The later user instruction explicitly directed execution of all seven WOs and superseded the initial planning-only boundary. This target-only implementation is recorded in the dedicated commit above. Validation was a fast, narrow scope comparison; it did not verify facts, links, Markdown, preservation completeness, policy correctness, or tests. No separate design-review agent, formal design review, or design approval is recorded.

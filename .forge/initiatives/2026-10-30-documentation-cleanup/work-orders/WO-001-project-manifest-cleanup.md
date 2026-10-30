# Work Order: WO-001 — Project Manifest Cleanup

## Metadata

- **Status:** `in-review`
- **Work-order kind:** `document-only`
- **Assignee:** Forge implementer; session/agent ID was not exposed in the execution environment.
- **Order/dependencies:** No predecessor WO. Independent design review and required human design approval precede execution; orchestrator provides the approved assignment and baseline evidence. No dependent implementation WO is identified.
- **Initiative:** [`../documentation-cleanup-v1.md`](../documentation-cleanup-v1.md)
- **Generated filename:** `WO-001-project-manifest-cleanup.md`
- **Owner/orchestrator:** Forge orchestrator
- **Validation:** [`../validations/validation-WO-001.md`](../validations/validation-WO-001.md) — `fail`; historical foundation validation is misdated in the manifest's completion evidence (see independent validation artifact).
- **Validation outcome:** `fail` — retain `in-review` / incomplete until the incorrect historical evidence date is corrected in authorized scope and independently revalidated.
- **Fixes:** none
- **Implementer session/agent ID:** not exposed in the execution environment.
- **Dedicated work-order commit:** `829bfeb46ed5148f70c53310be5715307af4d8dc` (single parent: `548034b701929b3da109a2c8aae5578d710d7538`).
- **Documentation-first relationship:** none; manifest cleanup is not a target for a dependent code change.
- **Commit description convention:** `docs-first (pre-implementation): no dependent implementation WOs — project manifest cleanup`; the conventional label does not assert a code dependency.
- **Session exception:** none

## Summary

Make the project manifest a clear, trustworthy project-policy and baseline reference without WO-specific commentary or a revision-history table. Preserve substantive policy, decisions, uncertainty, and traceable initialization evidence; apply the user's explicit removal of recovery-snapshot intent while retaining unrelated project policy and save safety; do not infer that unverified checks passed.

## In scope

- Edit only [`project-manifest.md`](../../../project-manifest.md): clarify formatting, refresh or qualify stale initialization-era baseline facts, and remove WO-specific narrative and revision-history entries.
- Preserve initialization completion, project intake and non-goals, guided working agreement, technical constraints, project policies and safeguards, roadmap revision rule, still-open decisions and owners, and meaningful initialization/review evidence.
- Remove recovery-snapshot capability language from the manifest's desired product/technical constraints in accordance with the user decision; retain native-save, atomic-replacement, and failure-safety policy, and do not imply existing recovery code is removed.
- Keep a concise dated baseline and links to existing evidence; retain current commands as project policy without representing old runs as current results.

## Out of scope

- Product or architecture changes beyond the user's explicit recovery-snapshot-intent removal, source/test edits, requirements or blueprint edits, changes to process or historical artifacts, broad repository audit, and new roadmap or project-policy decisions.
- Inventing current branch, remote, PR, operating-system, export, framework approval, check results, or resolved decisions; do not weaken a policy in the name of cleanup.

## Requirements and blueprints

- No product requirements or architecture contracts are being changed outside the user's explicit removal of recovery-snapshot intent from the manifest; its other durable responsibilities and policy remain governed by [`_index.md`](../../../_index.md) and [`01-init-existing.md`](../../../process/01-init-existing.md).

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
- **Acceptance:** Given the existing manifest, linked evidence, and user decision, when the scoped edit is reviewed, then no obsolete initialization snapshot reads as an unqualified current fact and no recovery-snapshot capability remains as intended behavior; native-save/failure safety and unrelated project policy are retained; commands are not falsely reported as freshly passing; no revision table or WO-specific notes remain; baseline, unresolved decisions, historical handoff and initialization-review traceability remain clear and intact; and no other durable document or product behavior has changed.
- **Independent validation:** After applicable approval and execution, a fresh validator receives this WO and its exact dedicated single-parent commit hash, checks the exact commit/diff and these criteria, and records the result in the validation artifact. Do not validate dependent code or infer a pass from drafting.

## Durable-document impact

- **Project manifest:** `update` — only authorized durable target.
- **Requirements impact:** `not applicable` — no user-facing behavior or product intent is changed.
- **Blueprint impact:** `not applicable` — no architecture or technical contract is changed.
- **Documentation/code lag:** none intended; no dependent implementation WO.

## Completion checklist

- [x] Scoped manifest edit and preservation checks completed; no unsupported fact or silent policy change.
- [x] Document checks and applicable gates recorded, including unavailable or failed checks.
- [ ] Orchestrator records dispatch/completion identifiers, changed files, execution evidence and the exact dedicated WO commit hash, or the applicable no-change/Git disposition.
- [x] Fresh independent validator received this WO and exact hash; the fail outcome is recorded in [`validation-WO-001.md`](../validations/validation-WO-001.md). The work order remains incomplete pending correction and revalidation.

## Handoff

The manifest edit and dedicated commit are recorded above. The orchestrator should dispatch a fresh validator with this WO and the exact commit hash; compare only the assigned manifest change against this WO's acceptance criteria and record the result in the linked validation artifact. Do not validate dependent code or treat this implementer's checks as independent validation. Preserve the user-authorized source TODO and all other initiative artifacts outside the WO commit.

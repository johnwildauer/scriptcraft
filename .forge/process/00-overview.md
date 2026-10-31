# Software Forge Process Overview

## Purpose and routing

This directory provides practical workflow guidance. Start with the user's request and actual repository content; read only the relevant process document. [`../_index.md`](../_index.md) is the installed entry point.

Initialization is available when useful or requested, not a mandatory first step. Inspect the repository and actual contents of [`../project-manifest.md`](../project-manifest.md), if present, to understand what is known. Do not use a persisted completion status to decide whether to initialize. Use [`01-init-new.md`](01-init-new.md) for a new repository or [`01-init-existing.md`](01-init-existing.md) for an existing one when baseline work is needed.

## Durable documents

- [`../requirements/`](../requirements/) describes desired product behavior, users, and rationale.
- [`../blueprints/`](../blueprints/) describes desired technical structure and contracts in established-state voice. It is a target-state blueprint, not an inventory of current implementation.
- [`../initiatives/`](../initiatives/) coordinates bounded changes and records their plans, work orders, implementation, validation findings, and closeout.

Requirements and blueprints contain durable desired state, not initiative status, progress, history, or implementation evidence. A focused documentation work order may precede implementation when the desired target meaningfully changes; it is not a synchronization requirement when there is no durable impact. Existing code may lag the target. Raise unexpected, relevant mismatches to the orchestrator without broad auditing or scope expansion.

## Normal work pattern

Use [`02-initiative.md`](02-initiative.md) to plan the requested change as discrete, bounded work orders. Use [`04-work-order.md`](04-work-order.md) to implement each work order, [`05-validation.md`](05-validation.md) for its fresh validation, and [`06-closeout.md`](06-closeout.md) for applicable closeout. No default human design-approval gate is required. The user's actual direction may shape a suitable path; applicable external rules remain in force.

The orchestrator coordinates work and handles dependencies, problems, policy or external-rule applicability, and decisions for the user. Do not fabricate confirmation or turn uncertain evidence into a pass. Communicate actual blockers and uncertainty to the orchestrator.

## Attribution and validation

Implementers should inspect working changes before staging, isolate their attributable work-order changes, and preserve unrelated user work. When Git is available and the work can be safely isolated, commit the work-order changes before validator dispatch and report the implementation commit/diff to the orchestrator. Do not claim, stage, or commit unrelated changes. A separate worktree can help when practical; do not impose elaborate branch or history requirements.

For each completed discrete document or implementation work order, the implementer commits safely attributable changes before validation when Git is available. The orchestrator dispatches exactly one fresh coding validator with this same instruction for either kind of work:

> Load the work order and its attributable Git diff, confirm loose alignment, record the finding, and commit the finding separately.

The finding commit is separate from the implementation commit/diff. If Git is unavailable, no attributable changes exist, or safe attribution is not possible, still dispatch exactly one validator and record that the evidence does not support an alignment confirmation; commit the finding separately when Git permits. Do not manufacture validation or a commit. The validator assesses only loose alignment between the work order and its attributable diff—not requirements or blueprints as gates, lifecycle labels, broad project gates, or unrelated work. The orchestrator handles other evidence, dependencies, applicable external rules, actual issues, and user decisions.

# Validation

For every completed discrete document or implementation work order, the
orchestrator dispatches exactly one fresh coding validator with this same prompt:

> Load the work order and its attributable Git diff, confirm loose alignment, record the finding, and commit the finding separately.

The implementer commits safely attributable work-order changes before dispatch
when Git is available. The validator considers only the work order and that
implementation commit/diff; the finding commit must be separate from it. Record a
finding grounded in the available evidence; do not invent a pass. If Git is
unavailable, no attributable changes/diff exist, or attribution is unsafe, still
dispatch the single validator and record that alignment could not be confirmed.
Commit the finding separately when Git permits, without unrelated edits. Never
manufacture a finding commit or turn missing evidence into a workflow status gate.
Use
[`../initiatives/YYYY-MM-DD-initiative-template/validations/validation-template.md`](../initiatives/YYYY-MM-DD-initiative-template/validations/validation-template.md)
for the finding.

The orchestrator and implementer own checks, applicable user instructions,
external rules, progress, ordering, and other project gates. The orchestrator
handles genuine missing information or a material mismatch without automatic
fix-order churn.

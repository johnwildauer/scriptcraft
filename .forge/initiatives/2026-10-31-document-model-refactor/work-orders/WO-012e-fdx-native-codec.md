# Work Order: WO-012e — Reconcile the FDX-native reader and writer

- **Status:** implementation recorded in commit `e751810558d6815c896609ca709fdde701d97c3b`; user-owned review and validation have not been performed for this cutover.

## Outcome and scope
Replace obsolete unsupported-app-state checks with FDX-native validation and source ownership while retaining strict safety checks for actual content and unsupported structures.

## Components and implementation
- Update [FDX codec](../../../../crates/scriptcraft-core/src/fdx.rs), [preservation](../../../../crates/scriptcraft-core/src/preservation.rs) and privacy-safe synthetic fixtures/tests. Reader supplies ephemeral identities and runtime settings without treating them as source values; writer serializes actual FDX content and preserved source, not legacy metadata/profile/policies or private extensions. Remove only comparisons for state retired in WO-012b–d.
- Preserve original ordered title, DocumentRef/XRef and timestamps, PageLayout, FontSpec, ElementSettings, foreign paragraph IDs, unknown attrs/namespaces, runs, scene attachments and boards/image internals. Do not rewrite source timestamps or guess mappings. Preserve existing bounded parsing, XXE denial, XML validity, semantic re-read and source comparison. Maintain explicit safe errors for constructs not safely represented; older-version exception remains separate.
- For new documents emit only a minimal supported FDX structure based on evidence; generated source IDs are distinct from ephemeral keys. Do not claim external-app acceptance. Exercise settings visibility/page-start and nondefault runtime state without claiming exact FDX geometry.

## Acceptance and checks
- New, no-op and supported text edits round-trip roles, Unicode, runs, supported settings, title and opaque source; source metadata and synthetic board/image links remain equivalent at documented semantic granularity. Transient policy/geometry/IDs do not cause unsupported-state errors or appear as private content. Unknown unsafe input returns a typed error and no output.
- Run `cargo test -p scriptcraft-core fdx`, `cargo test -p scriptcraft-core preservation`, `cargo test --workspace`, and applicable format/lint/build checks. Fixtures contain synthetic image data only and no recorded local paths; no real-app result is claimed.

## Risks and dependency
Depends on WO-012d and WO-006–008. One attributable commit and implementer checks only. Risk: deleting an unsupported-state check that actually protects authored data; retain identity/reference and ownership guards. Structural edits remain WO-012f.

The earlier status was stale relative to the implementation commit present in the inspected Git history and the corresponding current source. Its implementation-result/check record above is historical evidence only; no check, review, or validation was run or re-asserted during WO-012 execution.

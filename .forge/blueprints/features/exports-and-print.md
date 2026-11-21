# Feature-Area Blueprint: Exports and Print

> This feature composes immutable project snapshots, fixed Print geometry, output encoders, and native host adapters for explicitly requested review output.

## Scope and requirements

- **Requirements:** [Exports and print](../../requirements/features/exports-and-print.md); [Dual dialogue](../../requirements/features/dual-dialogue.md) owns `REQ-WRITING-LOOP-013` for `AC-WRITING-013.7`.
- **Technical context:** [Technical context](../technical-context.md)
- **Requirements traced:** `REQ-WRITING-LOOP-004`, `REQ-WRITING-LOOP-013`; `AC-WRITING-004.1`–`AC-WRITING-004.6`, `AC-WRITING-013.7`.
- **Area summary:** PDF, plain text, and supported native printing derive from Print-purpose content and never substitute for or mutate the native FDX source.

## Shared composition

- [Layout and pagination](../components/layout-and-pagination.md) owns Print composition and canonical document-point geometry driven by the current definitions and resolved typography; [Editing session](../components/editing-session.md) supplies a stable project snapshot and owns native storage; [FDX preservation and storage](../components/fdx-preservation-and-storage.md) retains the codec/source boundary, with canonical and retained source values owned by [Document Model](../components/document-model.md).
- [Command contract](../components/command-contract.md) carries PDF/Print activations and options; [Desktop adapters](../components/desktop-adapters.md) owns native destination/print panels and external operations.
- The reusable bounded encoders belong to [`scriptcraft-export/src/lib.rs`](../../../crates/scriptcraft-export/src/lib.rs), with plain-text output and loss reporting in [`text.rs`](../../../crates/scriptcraft-export/src/text.rs) and PDF construction in [`pdf.rs`](../../../crates/scriptcraft-export/src/pdf.rs). The export package depends on model/layout and consumes immutable model plus Print-layout data; it owns no session mutation, destination picker, native print panel, filesystem publication policy, or platform adapter. This is package ownership within the feature composition, not a separate export component or navigation entry.
- `scriptcraft-layout` and `scriptcraft-export` have a zero-permanent-test and zero-test-only-fixture-directory policy. Their production layout/environment/cache/geometry and text/PDF contracts below remain authoritative; test removal is not validation or acceptance evidence. Canonical Print composition, output fallback/reporting limits, and desktop/native print evidence boundaries are unchanged and remain user-owned for evidence decisions. Follow the [prevention guidelines](../../../.plan/2026-11-20-template-typography-pruning/future-agent-guidelines.md).

## Capability realization

### PDF and plain-text review copies

- **Requirements:** `REQ-WRITING-LOOP-004`; `AC-WRITING-004.1`–`AC-WRITING-004.5`, `AC-WRITING-013.7`.
- **Composition and flow:** Save As PDF enters the typed host-effect path. On supported systems the native panel returns one accepted destination plus a fresh Include Title Page choice; only then does the adapter request a Print composition from an immutable project snapshot. The PDF encoder consumes canonical Print positions and dynamic resolved typography; Base-14 font substitutions, unsupported color handling, and glyph/metric substitutions are exposed through bounded reports. Plain text emits display text from Print lines and reports its lossy nature. Absent color is black in output, supported explicit colors remain literal, and unsupported explicit colors are reported rather than silently claimed equivalent. These are bounded, noncanonical review outputs, not replacements for the native FDX source. Dual-dialogue Print composition is independent of transient editor group selection, and alternate payloads never enter output.
- **Boundary:** Initial dialog cancellation performs no layout or write. Output writes only to the accepted destination and cannot update session dirty/baseline/history, selection, zoom, or open views. Output uses canonical Print geometry without a second wrap/repagination policy. The export package does not claim universal Unicode/font support, exact visual parity, or physical print delivery. Platform limitations are explicit rather than replaced by a competing options workflow.

### Native Print operation

- **Requirements:** `AC-WRITING-004.6`, `AC-WRITING-013.7`.
- **Composition and flow:** The parameterless Print command is accepted only by the supported host. The native panel gathers platform settings and title-page choice; cancellation terminates before output construction. Acceptance binds settings and an immutable request-time snapshot, requests canonical Print composition, builds the output once, and submits one native print operation. macOS uses the PDF-backed print path; Windows uses resolved host fonts/metrics and reports bounded substitutions where applicable. The adapter reports cancellation/failure/success only at the level the host API supports.
- **Boundary:** Print never routes through viewport painting, a temporary user-visible export, or a second settings panel. A reported operation success does not claim exact typography or physical delivery; unsupported platforms return their explicit unavailable result.

## Decisions

- Share canonical, document-driven Print layout across PDF, text, and native printing; do not use transient viewport geometry or output-side reflow.
- Keep native panels and operation lifetimes in desktop adapters while keeping document and session state read-only for export.

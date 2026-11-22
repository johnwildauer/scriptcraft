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

- **Requirements:** `REQ-WRITING-LOOP-004`; `AC-WRITING-004.1`–`AC-WRITING-004.7`, `AC-WRITING-013.7`.
- **Composition and flow:** Save As PDF enters the typed host-effect path. On supported systems the native panel returns one accepted destination plus a fresh Include Title Page choice; only then does the adapter request a Print composition from an immutable project snapshot. PDF receives one coherent resolved-font environment plus immutable exact face/byte payload. Packaged Courier Prime font programs are embedded complete and unmodified under OFL; eligible host/synthetic faces follow bounded licensing, format, metric, repertoire, and byte-budget policy. The shared complete-face encoder normalizes PDF FontDescriptor ascent, descent, cap height, and FontBBox coordinates from each face's actual units per em to 1000ths of an em; these PDF-unit metadata values do not rewrite the embedded font-program bytes. CID/GID/ToUnicode cover non-BMP mappings; missing embedded glyphs use GID 0 and are reported. Base-14 and `?` substitution are host-only fallbacks; packaged payload failures are errors. PDF bounds are 8 MiB per face, 24 MiB aggregate original font bytes, 4,096 scalars per face, 268 resources, 64 MiB output, and 16 report categories. Plain text emits display text from Print lines, remains environment-only, and reports its lossy nature. Absent color is black, supported explicit colors remain literal, and unsupported explicit colors are reported. Dual-dialogue Print composition is independent of transient editor group selection, and alternate payloads never enter output.
- **Boundary:** Initial dialog cancellation performs no layout or write. Output writes only to the accepted destination and cannot update session dirty/baseline/history, selection, zoom, or open views. Output uses canonical Print geometry without a second wrap/repagination policy. The export package does not claim universal Unicode/font support, exact visual parity, or physical print delivery. Platform limitations are explicit rather than replaced by a competing options workflow.

### Native Print operation

- **Requirements:** `AC-WRITING-004.6`, `AC-WRITING-013.7`.
- **Composition and flow:** The parameterless Print command is accepted only by the supported host. The native panel gathers platform settings and title-page choice; cancellation terminates before output construction. Acceptance binds settings and one immutable request-time snapshot, requests canonical Print composition, builds the output once, and submits one native print operation. macOS PDF-backed Print retains the same Arc-backed environment-plus-payload snapshot used by PDF export; before its panel, the macOS adapter initializes paper size from the snapshot's effective dimensions, zero additional application margins, 100% native scaling, and horizontal/vertical centering off, while PDFKit adds no fit scaling. Non-centered placement is a pre-panel default, not a guarantee about accepted imageable bounds or absence of every host-specific translation. These are defaults only, and accepted paper, orientation, and scaling choices pass through unchanged. Windows receives environment-only inputs and draws immutable vector packages from the canonical baseline (line-box top plus retained offset) and scalar x; a measured no-wrap DirectWrite baseline only cancels the native row offset, and preview/final share the package/drawing path without line-box y clipping. Natural extents are compared with the actual leading-sized line box and remain observations; descent-aware fit belongs to WO-002.1. The adapter reports bounded family, face/shaping/coverage, missing-glyph, natural-extent and color observations, not canonical geometry uncertainty. The adapter reports cancellation/failure/success only at the level the host API supports.
- **Boundary:** Print never routes through viewport painting, a temporary user-visible export, or a second settings panel. A reported operation success does not claim exact typography or physical delivery; unsupported platforms return their explicit unavailable result.

## Decisions

- Share canonical, document-driven Print layout across PDF, text, and native printing; do not use transient viewport geometry or output-side reflow.
- Keep native panels and operation lifetimes in desktop adapters while keeping document and session state read-only for export.

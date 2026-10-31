# Work Order: WO-012d — Derive runtime layout without legacy document profile

- **Status:** `draft / not started`; depends on WO-012c.

## Outcome and scope
Replace persistent ScriptCraft page-profile ownership with explicit runtime layout inputs while preserving actual FDX settings and deterministic editor/PDF behavior.

## Components and implementation
- Refactor [model](../../../../crates/scriptcraft-core/src/lib.rs), [layout](../../../../crates/scriptcraft-core/src/layout.rs), [exports](../../../../crates/scriptcraft-core/src/exports.rs), desktop renderer/font and tests. Cover name/version, paper dimensions, all margins, character/line counts, line advance, gap, family/size/width and all 17 role boxes. Use existing deterministic defaults where a source mapping is unproven; test alternate runtime inputs without promising they survive reopen.
- Separate source-derived visibility and new-page overrides from app geometry: retain the existing explicitly recognized settings and their source XML. For unsupported or ambiguous semantics preserve source and return a safe explicit limitation where needed; never silently print known hidden text. Do not reinterpret FDX dimensions/fonts until units and semantics are tested.
- Cache keys include effective runtime inputs and source-derived settings; zoom remains presentation-only. Transitional JSON must remain lossless or reject safely, with no silent conversion of nondefault geometry. Record that pagination can differ after legacy reopen or for unimplemented FDX layout settings.

## Acceptance and checks
- Stable geometry, hit testing, PDF and pagination for identical effective inputs; viewport zoom independence; cache invalidation; all 17 roles, recognized hidden/page-start behavior, unsupported setting handling and source retention. Nondefault runtime configuration is not written as private state.
- Run `cargo test -p scriptcraft-core layout`, `cargo test -p scriptcraft-core exports`, `cargo test -p scriptcraft`, `cargo test --workspace`, and applicable format/lint/build checks. Record exact results.

## Risks and dependency
Depends on WO-012c and WO-005. One attributable commit, implementer checks only. Risk: regression in font metrics or silently displaying hidden content. Hand effective-input/source-settings contract to WO-012e.

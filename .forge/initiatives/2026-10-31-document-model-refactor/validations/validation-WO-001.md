# Finding: WO-001 — Bounded-safety evidence incomplete

- **Work order:** WO-001 — [FDX mapping and safety evidence](../work-orders/WO-001-mapping-and-safety-evidence.md:1)
- **Attributable implementation commit/diff:** `13b488f43001e2c86910c71b8b3a6a76d12fdac4` (single parent `01b768dd1e1a7920d271405eb83c1ed84784b370`); the exact diff adds only the mapping report and two synthetic XML fixtures under `evidence/`.

## Finding

**FAIL — required acceptance evidence is incomplete.** The attributable change is correctly bounded to WO-001 evidence: it changes no Rust source, Cargo manifest, native JSON behavior, or application behavior. The mapping report is substantial and accurately distinguishes observed facts, candidate mappings, blockers, semantic preservation, and unproven interoperability. It inventories the current model/profile fields, all 17 directed role labels, major FDX source sections and ownership boundaries, ID domains, preservation strategy alternatives, unsafe-edit cases, a minimal-template candidate, and version/unknown-dependency limitations. It does not present unresolved homes for ScriptCraft identity/schema/profile/edit-policy state or unknown XML dependencies as solved.

The decisive gap is the WO-001 implementation instruction to provide **configurable tested bounds**. The report proposes byte, event, depth, attribute, text, and image caps, but states these are not parser-tested and that no production parser or resource-limit test was run. Proposed numbers and a future test plan are not evidence that the required bounds are tested. The synthetic fixtures are also illustrative inputs, not parser/security tests. This is missing acceptance evidence, distinct from the explicitly disclosed FDX mapping and interoperability blockers that WO-001 was meant to identify and pass forward.

The fixture evidence is useful but deliberately limited: sanitized structural inspection confirmed both fixtures are well-formed, the preservation fixture includes all 17 distinct requested role labels plus a namespaced opaque child, multiple text runs, nested synthetic scene data, and synthetic board/item/image identifiers; neither contains a path attribute or nonempty image payload. The minimal fixture has only a simple title-page paragraph, and the fixtures do not exercise parser behavior, proposed limits, or the full range of observed source-section shapes. No Final Draft compatibility claim is supported.

## Independent checks and results

- `git show -s --format='commit=%H%nparent=%P%nsubject=%s' 13b488f43001e2c86910c71b8b3a6a76d12fdac4` confirmed the requested exact commit and parent. The diff lists exactly three added evidence files; `git diff --check 01b768dd1e1a7920d271405eb83c1ed84784b370 13b488f43001e2c86910c71b8b3a6a76d12fdac4` **passed**.
- Sanitized Python `xml.etree.ElementTree` structural parsing of `.reference/reference.fdx` and `.reference/reference2.fdx` emitted only root/section counts, paragraph type labels, attribute names, and boolean path/image-payload presence; it displayed no embedded image data or path values. Both references parsed. The populated reference has 34 descendant paragraphs in Content: 17 distinct typed labels and 3 without Type. This is structural observation only, not schema or application validation.
- The same parser checked the two synthetic fixtures: both parsed; all 17 directed labels and the report's claimed selected shapes were present; neither fixture had a path attribute or nonempty image text. **Pass for fixture shape only.**
- `cargo fmt --all -- --check` — **passed**.
- `cargo clippy --workspace --all-targets -- -D warnings` — **passed**.
- `cargo test --workspace` — **passed**, 231 tests passed, 0 failed.
- `cargo build --workspace` — **passed**.
- `cargo check --workspace` — **passed**.

The Cargo checks demonstrate the unchanged workspace remains healthy; they do not validate FDX parsing, preservation, resource bounds, or interoperability. No identifiable Final Draft version/platform test or production FDX parser test was performed in this validation.

## Disposition and readiness

- **Pass:** attributable scope and commit provenance; substantive field/section inventory; explicit candidate-versus-blocker distinctions; privacy-safe synthetic fixture shapes; unchanged-code/build acceptance; reproduced workspace checks.
- **Warning / intended limitations passed forward:** no proven durable home for all ScriptCraft-only identity/schema/profile/policy state; unverified unknown-reference semantics and opaque-content edit transformations; candidate-only parser, encoding, and resource-budget policies; no verified minimal Final Draft template or real-application round trip. These are explicitly recorded limitations, not findings that the supplied references establish otherwise.
- **Fail / missing acceptance evidence:** tested configurable FDX bounds and parser-level boundary/security evidence are absent. No finding of an actual runtime vulnerability is made because no parser implementation was added or tested.
- **WO-002 readiness:** **not cleared by this validation.** Do not start WO-002 on the assumption that WO-001 is accepted or that the identified blockers are resolved. Any separately authorized model-only scaffolding must retain the initiative's transitional restrictions and cannot claim safe FDX native save/reopen; the missing WO-001 tested-bounds evidence and explicit blocker handoff require disposition before the dependent sequence proceeds.

Reviewed artifacts: [mapping and safety evidence](../evidence/WO-001-mapping-and-safety-evidence.md:1), [minimal synthetic fixture](../evidence/wo001-minimal-v6-synthetic.xml:1), [preservation-shapes synthetic fixture](../evidence/wo001-preservation-shapes-synthetic.xml:1), and [approved initiative handoff](../document-model-refactor-v1.md:197).

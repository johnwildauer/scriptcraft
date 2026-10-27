# Validation — WO-004 Role Casing Policy (v2)

## Result

- **Validation result:** `PASS WITH WARNINGS`
- **Validated work order:** [`WO-004-role-casing-policy.md`](../work-orders/WO-004-role-casing-policy.md:1)
- **Initiative:** [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:1)
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `architect-fresh-context-editor-geometry-corrections-v2-WO-004-validation-2026-10-26`
- **Session exception:** `none`
- **Work-order state at validation:** `in-review`; this validator did not change the state
- **Fix chain:** `none`
- **Product source modified by validator:** `no`

## Fresh comparison basis and boundary

I independently read the validation rules in [`05-validation.md`](../../../process/05-validation.md:1), the authoritative v2 plan in [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:73), the reconciled design review in [`design-review-v2.md`](../reviews/design-review-v2.md:146), the approved work order and its implementation handoff, the v2 execution log, the applicable requirement and blueprints, the current model/editing/persistence source, the focused casing tests, and the available changed-file/equivalent evidence.

The comparison is meaningful. The work order is `in-review`; its changed-file record names [`lib.rs`](../../../../crates/scriptcraft-core/src/lib.rs:154), [`editing.rs`](../../../../crates/scriptcraft-core/src/editing.rs:138), and [`wo004_role_casing.rs`](../../../../crates/scriptcraft-core/tests/wo004_role_casing.rs:1); the execution log records the matching dispatch, implementation, checks, and status transition at [`execution-log.md`](../execution-log.md:74); and the current source contains the reported persisted setting, decoder, edit normalization, mapping, and transactional history paths.

This validation assesses only persisted uppercase-cue compatibility, legacy decode, no-open-rewrite, insertion/paste/role-transition normalization, Unicode expansion and caret/selection mapping, undo/redo, non-cue preservation, scope, durable impact, and the required Cargo gates. It does not validate WO-005 parenthetical behavior, WO-006 responsiveness, native UI behavior, unrelated geometry/font work, or future settings UI.

## Evidence assessment

| Boundary | Status | Fresh evidence and assessment |
|---|---|---|
| Persisted uppercase-cue compatibility | **PASS** | [`PageProfile`](../../../../crates/scriptcraft-core/src/lib.rs:154) contains the bounded boolean `uppercase_cues`; [`PageProfile::default()`](../../../../crates/scriptcraft-core/src/lib.rs:175) sets it to `true`; [`encode_envelope()`](../../../../crates/scriptcraft-core/src/lib.rs:379) serializes it in the existing schema; and [`decode_project()`](../../../../crates/scriptcraft-core/src/lib.rs:618) restores it without introducing a schema bump. A disabled value remains representable and the focused test exercises disabled editing behavior at [`disabled_policy_and_non_cue_roles_preserve_source_casing()`](../../../../crates/scriptcraft-core/tests/wo004_role_casing.rs:64). |
| Legacy decode | **PASS** | [`optional_bool_field()`](../../../../crates/scriptcraft-core/src/lib.rs:574) returns the enabled default only when the field is absent and returns [`ModelError::WrongType`](../../../../crates/scriptcraft-core/src/lib.rs:562) for a malformed present type. The unit regression [`legacy_profile_missing_uppercase_policy_defaults_true()`](../../../../crates/scriptcraft-core/src/lib.rs:673) and the focused integration coverage at [`policy_persists_and_legacy_missing_field_defaults_true_without_open_rewrite()`](../../../../crates/scriptcraft-core/tests/wo004_role_casing.rs:77) both establish the missing-field compatibility rule. |
| No-open-rewrite | **PASS** | [`Project::from_json()`](../../../../crates/scriptcraft-core/src/lib.rs:263) decodes and validates semantic text without invoking editing normalization. The focused round trip keeps `saved lower text` unchanged after reopen and after legacy decode at [`wo004_role_casing.rs`](../../../../crates/scriptcraft-core/tests/wo004_role_casing.rs:79). No open/decode path calls [`normalize_element()`](../../../../crates/scriptcraft-core/src/editing.rs:436). |
| Insertion and paste normalization | **PASS** | [`EditingSession::insert()`](../../../../crates/scriptcraft-core/src/editing.rs:138) performs replacement inside [`mutate()`](../../../../crates/scriptcraft-core/src/editing.rs:325), then normalizes the resulting element. [`insert_paste()`](../../../../crates/scriptcraft-core/src/editing.rs:150) uses the same transaction. The focused test proves Scene Heading and Character ASCII insertion, paste, and resulting caret offsets at [`insertion_and_paste_normalize_cues_and_map_unicode_expansion()`](../../../../crates/scriptcraft-core/tests/wo004_role_casing.rs:20). |
| Role assignment and transitions | **PASS** | [`change_role()`](../../../../crates/scriptcraft-core/src/editing.rs:286) normalizes after assigning the resulting role; [`tab()`](../../../../crates/scriptcraft-core/src/editing.rs:247) applies the same rule for Tab transitions; and [`split_at()`](../../../../crates/scriptcraft-core/src/editing.rs:486) normalizes a newly created cue element for Enter transitions. The focused regression covers Action → Character with existing `straße`, undo/redo, and Transition → Scene Heading via Tab at [`role_assignment_and_tab_transition_normalize_existing_text_transactionally()`](../../../../crates/scriptcraft-core/tests/wo004_role_casing.rs:41). |
| Unicode expansion and caret/selection mapping | **PASS** | [`uppercase_with_mapping()`](../../../../crates/scriptcraft-core/src/editing.rs:420) iterates Unicode scalar values with Rust [`char::to_uppercase()`](../../../../crates/scriptcraft-core/src/editing.rs:427), accumulates the transformed UTF-8 byte length for the prefix, and rejects non-boundary offsets. Both insertion and role-change paths use the mapped byte boundary. Focused evidence covers `ǰ → J̌`, `ß → SS`, prefix insertion, and resulting offsets at [`wo004_role_casing.rs`](../../../../crates/scriptcraft-core/tests/wo004_role_casing.rs:29) and [`wo004_role_casing.rs`](../../../../crates/scriptcraft-core/tests/wo004_role_casing.rs:43). |
| Undo/redo and transactional safety | **PASS** | Normalization occurs before [`mutate()`](../../../../crates/scriptcraft-core/src/editing.rs:331) validates the candidate and records one snapshot; successful changes therefore enter the existing history and redo boundary, while validation errors occur on the clone before live state changes. The focused role-assignment test explicitly verifies undo restores `straße` and redo restores `STRASSE` at [`wo004_role_casing.rs`](../../../../crates/scriptcraft-core/tests/wo004_role_casing.rs:48). The implementation preserves the existing dirty/revision/history handling rather than adding parallel casing state. |
| Non-cue and disabled-policy preservation | **PASS** | [`is_uppercase_cue()`](../../../../crates/scriptcraft-core/src/editing.rs:416) limits normalization to Scene Heading and Character, and both normalizers return the original offset/selection when the policy is disabled or the role is non-cue. The focused regression proves disabled Scene Heading and enabled Action preserve `MiXeD` at [`disabled_policy_and_non_cue_roles_preserve_source_casing()`](../../../../crates/scriptcraft-core/tests/wo004_role_casing.rs:64). |
| Persistence round trip | **PASS** | [`serialize()`](../../../../crates/scriptcraft-core/src/persistence.rs:56) and [`deserialize()`](../../../../crates/scriptcraft-core/src/persistence.rs:63) use the project model representation, preserving the policy and existing text. The focused test verifies save/reopen preserves lower-case stored cue text and the enabled setting at [`wo004_role_casing.rs`](../../../../crates/scriptcraft-core/tests/wo004_role_casing.rs:77). |
| Scope and rogue-behavior check | **PASS** | The recorded implementation boundary is the model profile, transactional editing core, focused core tests, and authorized requirement/blueprint evidence. No settings UI, locale-specific title casing, formatting-only rendering, schema migration, second text buffer, or unrelated product area was observed. No deceptive, backdoor, malicious, nefarious, or otherwise rogue behavior was found. The execution log separately records that WO-005 and WO-006 were not implemented at [`execution-log.md`](../execution-log.md:74). |
| Durable-document impact | **PASS WITH NOTE** | The work order classifies the requirement and core blueprint as `update` at [`WO-004-role-casing-policy.md`](../work-orders/WO-004-role-casing-policy.md:37). The component blueprint records the persisted setting/no-open-rewrite/role-limited transactional contract at [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:71), and the feature requirement records the approved refinement at [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md:52). This matches the durable behavior; no schema migration or UI setting was added. |

## Required Cargo gates

The implementer and execution log record all required gates as passing: [`cargo fmt --all -- --check`](../../../../Cargo.toml:1), [`cargo check --workspace`](../../../../Cargo.toml:1), [`cargo test --workspace`](../../../../Cargo.toml:1) with 82 tests and 0 doc tests, [`cargo clippy --workspace --all-targets -- -D warnings`](../../../../Cargo.toml:1), and [`cargo build --workspace`](../../../../Cargo.toml:1), at [`WO-004-role-casing-policy.md`](../work-orders/WO-004-role-casing-policy.md:58) and [`execution-log.md`](../execution-log.md:78). The recorded evidence also explains the initial denied-test-lint failure and the typed-test remediation before the final pass.

These gates were **not independently rerun** in this tool-only validation context, per the requested evidence boundary. The available recorded results are specific, internally consistent with the source and focused test files, and do not constitute a required-gate failure. No native/manual gate is applicable to this core canonical-casing work order, and no native success is claimed.

## Warnings and notes

### W-001 — No standalone diff result was available

- **Severity:** warning
- **Evidence:** The available inspection tools exposed the changed-file records, execution log, current source, tests, and durable-document updates, but no standalone Git/branch diff output. The records are internally consistent and provide a meaningful file-level comparison; this is not a comparison blocker under [`05-validation.md`](../../../process/05-validation.md:20).
- **Recommendation:** Preserve the explicit changed-file boundary and attach the normal comparable branch/file diff if one becomes available. Do not represent this validation as having independently inspected a standalone patch.
- **Required next state:** Note only; no fix work order is required.

### W-002 — Cargo gates are recorded, not independently rerun

- **Severity:** warning
- **Evidence:** The implementer recorded final passing Cargo gates at [`WO-004-role-casing-policy.md`](../work-orders/WO-004-role-casing-policy.md:58), and the execution log repeats them at [`execution-log.md`](../execution-log.md:78). This validator had no command-execution tool and therefore did not rerun them.
- **Recommendation:** Retain the recorded results and the explicit independent-rerun limitation. The focused source/test inspection found no contradictory failure, and project policy does not make a separate native gate applicable here.
- **Required next state:** Note only; no fix work order is required.

## Final verdict and exact next state

- **Implementation alignment:** Pass. The persisted enabled-by-default policy, missing-field legacy compatibility, no-open-rewrite boundary, cue-only transactional normalization, Unicode transformed-prefix mapping, undo/redo behavior, non-cue/disabled preservation, and save/reopen behavior match the approved v2 oracle at [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:79) and [`WO-004-role-casing-policy.md`](../work-orders/WO-004-role-casing-policy.md:45).
- **Hard-stop finding:** None.
- **Fix work order required:** `no`.
- **Exact next state:** The orchestrator should link this artifact from [`WO-004-role-casing-policy.md`](../work-orders/WO-004-role-casing-policy.md:9) and [`execution-log.md`](../execution-log.md:80), set `WO-004 = complete` under [`05-validation.md`](../../../process/05-validation.md:79), retain W-001 and W-002 as warnings, and proceed to WO-005 in the per-work-order execution/validation cadence. Do not mark WO-005 or WO-006 complete from this result.

# Validation — WO-003 Long-Document Validity

## Validation metadata

- **Validation status:** `PASS WITH WARNINGS`
- **Work order:** [`WO-003-long-document-validity.md`](../work-orders/WO-003-long-document-validity.md:1)
- **Initiative:** [`ui-quality-of-life-v1.md`](../ui-quality-of-life-v1.md:1)
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `debug-fresh-context-ui-quality-of-life-WO-003-validation-2026-10-25T21:18:02Z`
- **Fresh-context statement:** This validation was performed as a separate fresh-context session. I did not implement WO-003, did not validate WO-004, and did not treat the implementer's report as proof without comparing it to the source, durable artifacts, diff evidence, and independently rerun checks.
- **Work-order state at validation:** `in-review`; WO-003 is not marked complete.

## Comparison basis

Inspected the validation rules in [`05-validation.md`](../../../process/05-validation.md:1) and lifecycle rules in [`00-overview.md`](../../../process/00-overview.md:1), then compared:

- The initiative scope, approved decisions, dependency order, and validation boundary in [`ui-quality-of-life-v1.md`](../ui-quality-of-life-v1.md:47).
- The approved fresh-context design review in [`design-review.md`](../reviews/design-review.md:1).
- WO-003's authorized scope, acceptance criteria, durable-document classification, changed-file record, and completion evidence in [`WO-003-long-document-validity.md`](../work-orders/WO-003-long-document-validity.md:16).
- Dispatch/completion evidence and the recorded check results in [`execution-log.md`](../execution-log.md:24).
- The implementation in [`ScreenplayLayout::build()`](../../../../crates/scriptcraft-core/src/layout.rs:86), [`wrap_element()`](../../../../crates/scriptcraft-core/src/layout.rs:334), [`Project::to_json()`](../../../../crates/scriptcraft-core/src/lib.rs:218), [`ScriptCraftApp::show_error()`](../../../../apps/scriptcraft/src/main.rs:165), and the layout error path in [`live_document_surface()`](../../../../apps/scriptcraft/src/main.rs:378).
- The integration fixtures in [`wo003_long_document.rs`](../../../../crates/scriptcraft-core/tests/wo003_long_document.rs:1), including bounded dimensions, newline variants, final-page caret, round trip, failed edit/layout, and JSON-bound tests.
- The resulting durable facts in [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:117) and [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:109).

The compared WO-003 implementation is limited to the authorized layout/model/app/fixture/blueprint surface. Existing adjacent WO-001/WO-002 changes are present in the baseline and were not reassessed. No clearly unrelated, deceptive, backdoor, malicious, or nefarious behavior was found. WO-004 was not inspected for validation.

## Diagnostic assessment

Potential sources considered were: (1) malformed or over-bound pasted input; (2) CR/LF delimiter handling creating phantom layout lines; (3) a model-validation boundary that differed between in-memory serialization and persistence; (4) partial mutation or stale derived layout after a failed operation; (5) page/caret geometry failing at multi-page boundaries; (6) save/reopen changing semantic content or durable state; and (7) an app error path that reported failure without preserving or presenting the last valid document. Controlled fixtures and source comparison reduce the likely original causes to the former CR/LF splitter defect, the missing model-level JSON-size check, and the previously inconsistent layout-error presentation. The implementation addresses those causes while retaining the existing bounds and schema.

## Acceptance assessment

| WO-003 obligation | Result | Evidence |
|---|---|---|
| Controlled reproduction and root cause for `model (invalid)` | **PASS** | The work-order evidence separates element count, encoded text size, wrapped-line/page count, and newline pattern. The fixture [`controlled_fixture_dimensions_are_independent_and_bounded()`](../../../../crates/scriptcraft-core/tests/wo003_long_document.rs:73) exercises those dimensions. Source comparison confirms the reproduced CRLF phantom-line cause was corrected in [`wrap_element()`](../../../../crates/scriptcraft-core/src/layout.rs:334), while valid model content that previously bypassed the model serialization JSON bound is rejected in [`Project::to_json()`](../../../../crates/scriptcraft-core/src/lib.rs:218). No limits were raised and schema version 1 remains unchanged. |
| LF/CRLF/standalone-CR handling | **PASS** | [`wrap_element()`](../../../../crates/scriptcraft-core/src/layout.rs:334) recognizes `\n`, `\r`, and `\r\n` and advances over the two-byte CRLF delimiter without putting `\r` into a wrapped line. [`pasted_newline_patterns_preserve_semantics_without_phantom_cr_lines()`](../../../../crates/scriptcraft-core/tests/wo003_long_document.rs:104) checks three lines and exact CRLF line text. |
| Bounded JSON/model validation | **PASS** | [`Project::validate()`](../../../../crates/scriptcraft-core/src/lib.rs:200) retains the existing element/text/string/profile checks; [`Project::to_json()`](../../../../crates/scriptcraft-core/src/lib.rs:218) now checks encoded JSON length against `MAX_JSON_BYTES`; persistence also retains byte-size checking in [`serialize()`](../../../../crates/scriptcraft-core/src/persistence.rs:56) and [`deserialize()`](../../../../crates/scriptcraft-core/src/persistence.rs:63). [`model_serialization_keeps_json_bound_even_when_text_is_valid()`](../../../../crates/scriptcraft-core/tests/wo003_long_document.rs:192) proves a valid-text/oversized-JSON case returns `StringTooLong`. |
| Failed-operation safety | **PASS WITH WARNING** | Transactional oversized edit rollback is directly checked by [`failed_edit_and_layout_leave_last_valid_state_usable()`](../../../../crates/scriptcraft-core/tests/wo003_long_document.rs:164), and [`EditingSession::mutate()`](../../../../crates/scriptcraft-core/src/editing.rs:308) validates a cloned project before commit. The same test verifies invalid layout input does not replace the valid session. Existing workspace tests also cover failed native replacement and failed open preservation in [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1450). There is no single WO-003 fixture that injects a failed save and then reopens the prior destination, so evidence breadth is a warning, not a hard-stop: the authorized persistence contract and existing tests provide meaningful comparison. |
| Multi-page layout and final-caret reachability | **PASS** | [`final_page_caret_and_native_round_trip_survive_long_paste()`](../../../../crates/scriptcraft-core/tests/wo003_long_document.rs:124) pastes 500 CRLF-separated lines, requires more than one page, and confirms [`ScreenplayLayout::caret_rect()`](../../../../crates/scriptcraft-core/src/layout.rs:143) resolves the final caret. The fixture also independently checks page growth and non-empty pages in [`controlled_fixture_dimensions_are_independent_and_bounded()`](../../../../crates/scriptcraft-core/tests/wo003_long_document.rs:73). |
| Save/reopen semantic preservation | **PASS** | The bounded multi-page fixture saves through [`atomic_save()`](../../../../crates/scriptcraft-core/src/persistence.rs:72), deserializes the bytes, compares the reopened project to the session project, and checks the original CRLF text remains unchanged in [`final_page_caret_and_native_round_trip_survive_long_paste()`](../../../../crates/scriptcraft-core/tests/wo003_long_document.rs:124). |
| Error presentation and recoverability | **PASS WITH WARNING** | [`ScriptCraftApp::show_error()`](../../../../apps/scriptcraft/src/main.rs:165) records the typed/debug error and sets `Operation failed; document unchanged`; the layout failure branch in [`live_document_surface()`](../../../../apps/scriptcraft/src/main.rs:378) now routes through that path before displaying the raw layout detail. Native visual confirmation was unavailable, so presentation is source/test evidenced rather than native-smoke evidenced. |
| Tests and required checks | **PASS** | All recorded automated commands were independently rerun successfully; exact results are below. The dedicated WO-003 integration target reported 5 passing tests. |
| Durable-document classification | **PASS** | WO-003 classifies requirements as `verify unchanged` and blueprints as `update`. The changed blueprint sections record the retained bounds, CR/LF rule, model serialization boundary, recoverability, fixture evidence, and unchanged schema. This matches the implementation diff and does not claim unbounded support. |

## Exact check results

Executed from the repository root on 2026-10-25:

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | **PASS**, exit 0 |
| `cargo check --workspace` | **PASS**, exit 0 |
| `cargo test --workspace` | **PASS**, exit 0 — 18 app tests, 24 core unit tests, 5 WO-003 integration tests, 3 E2E tests; 50 tests passed, 0 failed; doc-tests: 0 run, 0 failures |
| `cargo clippy --workspace --all-targets -- -D warnings` | **PASS**, exit 0 |
| `cargo build --workspace` | **PASS**, exit 0 |
| Native smoke: `cargo run -p scriptcraft` | **LAUNCHED, INTERACTION UNAVAILABLE** — `target/debug/scriptcraft` launched and remained running, but this validation context exposed no observable/interactive native window. The process was stopped after the bounded attempt. No native paste, scroll, caret, save, reopen, or error-presentation success is claimed. An initial attempt to use the macOS-unavailable `timeout` utility failed with exit 127; it was not treated as an application result. |

The native limitation is consistent with the implementer's recorded evidence and is not a newly introduced automated failure. Project policy records manual desktop checks as non-required, so this does not block validation.

## Findings

### UIQOL-WO003-W-001 — Native long-document smoke unavailable

- **Severity:** warning
- **Evidence:** The bounded [`cargo run -p scriptcraft`](../../../../Cargo.toml:1) attempt launched the binary, but no interactive native window was available to exercise paste, multi-page scrolling, caret reveal, save/reopen, or visible recovery presentation. The work order and execution log also explicitly record native interaction as unavailable at [`WO-003-long-document-validity.md`](../work-orders/WO-003-long-document-validity.md:89) and [`execution-log.md`](../execution-log.md:26).
- **Recommendation:** Preserve this limitation and obtain native smoke evidence only in a capable desktop session; do not represent the headless fixtures as native proof.
- **Required next state:** Note only; no fix work order required under the current policy in [`project-manifest.md`](../../../project-manifest.md:92).

### UIQOL-WO003-W-002 — Failed-save evidence is indirect rather than WO-003-specific

- **Severity:** warning
- **Evidence:** The dedicated fixture directly tests failed edit rollback and invalid layout preservation in [`failed_edit_and_layout_leave_last_valid_state_usable()`](../../../../crates/scriptcraft-core/tests/wo003_long_document.rs:164), plus successful save/reopen in [`final_page_caret_and_native_round_trip_survive_long_paste()`](../../../../crates/scriptcraft-core/tests/wo003_long_document.rs:124). The workspace suite separately exercises failed native replacement/open preservation through [`failed_named_save_preserves_path_file_and_dirty_state()`](../../../../apps/scriptcraft/src/main.rs:1450) and [`failed_open_does_not_replace_named_project()`](../../../../apps/scriptcraft/src/main.rs:1460), but WO-003 does not add a combined failed-save-then-reopen fixture.
- **Recommendation:** Retain the passing existing persistence tests and consider a future focused fixture if stronger evidence is desired; do not expand WO-003 solely for this optional breadth gap.
- **Required next state:** Note only; no fix work order required because the authorized acceptance behavior is covered by the combined source and workspace evidence and no required policy gate failed.

## Verdict and next state

- **Verdict:** `PASS WITH WARNINGS`
- **Implementation alignment:** Pass. The implementation satisfies the authorized WO-003 long-document validity scope, retains bounded safety, preserves semantic round trips, and aligns both blueprint updates with the final diff. No genuine hard-stop, unrelated, deceptive, backdoor, malicious, or nefarious behavior was found.
- **Orchestrator action:** The orchestrator should complete WO-003 under the pass-with-warnings rule in [`05-validation.md`](../../../process/05-validation.md:79), link this validation from the work order and execution log, and retain both warnings. It should not create a fix work order.
- **Status guard:** This validator did not mark WO-003 complete. WO-004 was not validated.

# Validation — WO-001 Role Spacing and Courier Default (v2)

## Validation metadata

- **Review type:** `validation`
- **Status:** `PASS WITH WARNINGS`
- **Validator:** Fresh-context Forge validator
- **Validator session/agent ID:** `architect-fresh-context-editor-geometry-corrections-v2-WO-001-validation-2026-10-26`
- **Session exception:** `none`
- **Date:** `2026-10-26`
- **Initiative:** [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:1)
- **Validated work order:** [`WO-001-role-spacing-and-courier-default.md`](../work-orders/WO-001-role-spacing-and-courier-default.md:1)
- **Canonical artifact conflict:** [`validation-WO-001.md`](validation-WO-001.md:1) already contains the historical v1 Vertical Edge Clamping validation. Per the user's instruction, it was not overwritten. This v2 validation is therefore recorded separately as [`validation-WO-001-v2.md`](validation-WO-001-v2.md:1).
- **Fix chain:** `none`

## Fresh-context comparison basis

I independently read the validation rules in [`05-validation.md`](../../../process/05-validation.md:1), the authoritative v2 plan in [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:1), the reconciled design review in [`design-review-v2.md`](../reviews/design-review-v2.md:146), the v2 work order, the execution log, applicable requirements, blueprints, [`format-rules.md`](../format-rules.md:1), and the changed source/test evidence. I did not modify product code, silently correct an artifact, validate later work orders, or infer native visual success from headless evidence.

The implementation comparison uses the changed-file record and implementation evidence in [`WO-001-role-spacing-and-courier-default.md`](../work-orders/WO-001-role-spacing-and-courier-default.md:63), the matching v2 dispatch/check entries in [`execution-log.md`](../execution-log.md:46), and direct inspection of [`layout.rs`](../../../../crates/scriptcraft-core/src/layout.rs:89), [`lib.rs`](../../../../crates/scriptcraft-core/src/lib.rs:154), [`exports.rs`](../../../../crates/scriptcraft-core/src/exports.rs:66), and the affected assertion in [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1669). No standalone Git diff was available through the inspection tools; the recorded changed-file/evidence set and current source provide a meaningful comparison, with that limitation retained as a warning.

## Authorized scope

The comparison boundary is limited to:

- incoming-role spacing: zero extra gap for every body predecessor when the incoming role is Dialogue or Parenthetical; `boundary_gap` for every other incoming body role; same-element `line_advance`; title-page exclusion; and logical page-capacity accounting;
- new-project [`PageProfile::default()`](../../../../crates/scriptcraft-core/src/lib.rs:174) selecting `Courier`;
- bounded compatibility for stored `Courier`, `Courier New`, and `Courier Prime`, including typed errors for absent or wrongly typed stored font fields;
- PDF mapping consuming the persisted family, with `Courier` emitting `/BaseFont /Courier`;
- authorized updates to [`format-rules.md`](../format-rules.md:32), [`screenplay-core.md`](../../../blueprints/components/screenplay-core.md:68), [`screenplay-writing-loop.md`](../../../blueprints/features/screenplay-writing-loop.md:89), and [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md:39).

Later v2 work orders, canonical text, role transitions, viewport zoom, pointer behavior, font packaging, fallback databases, arbitrary formatting, and schema migration were not assessed as WO-001 obligations.

## Acceptance and policy assessment

| Obligation or gate | Result | Fresh-context evidence |
|---|---|---|
| Exact incoming Dialogue/Parenthetical spacing oracle | **PASS** | [`role_boundary_gap()`](../../../../crates/scriptcraft-core/src/layout.rs:368) selects zero based on the incoming role. [`every_incoming_dialogue_or_parenthetical_has_no_extra_gap()`](../../../../crates/scriptcraft-core/src/layout.rs:648) covers all seven body predecessors and both incoming roles, asserting `line_advance`. |
| Other incoming roles retain `boundary_gap` | **PASS** | [`other_incoming_roles_use_boundary_gap()`](../../../../crates/scriptcraft-core/src/layout.rs:672) covers same-role and representative cross-role pairs, asserting `line_advance + boundary_gap`. |
| Exact point geometry and page-capacity behavior | **PASS** | [`place_element()`](../../../../crates/scriptcraft-core/src/layout.rs:241) adds the selected point gap directly and increments logical capacity only when the gap is nonzero. [`zero_gap_does_not_consume_page_capacity()`](../../../../crates/scriptcraft-core/src/layout.rs:692) covers the zero-gap boundary. [`non_multiple_boundary_gap_is_preserved_as_point_geometry()`](../../../../crates/scriptcraft-core/src/layout.rs:610) preserves non-integral-in-line-unit point semantics. |
| Same-element advance and title-page boundary exclusion | **PASS by inspected implementation/evidence** | Same-element wrapped lines advance by [`line_advance`](../../../../crates/scriptcraft-core/src/layout.rs:304); title-page elements are laid out separately before body placement in [`ScreenplayLayout::build()`](../../../../crates/scriptcraft-core/src/layout.rs:89). The work-order evidence records title-page exclusion and updated boundary fixtures. |
| New projects default to Courier | **PASS** | [`PageProfile::default()`](../../../../crates/scriptcraft-core/src/lib.rs:174) sets `font_family` to `Courier`; [`new_project_has_valid_start()`](../../../../crates/scriptcraft-core/src/lib.rs:641) asserts the default. |
| Supported stored families round-trip unchanged | **PASS** | [`supported_profile_fonts_round_trip_without_rewriting()`](../../../../crates/scriptcraft-core/src/lib.rs:648) covers `Courier`, `Courier New`, and `Courier Prime`. |
| Malformed/absent stored font fields remain typed errors | **PASS** | [`missing_or_malformed_font_fields_remain_typed_errors()`](../../../../crates/scriptcraft-core/src/lib.rs:659) asserts `MissingField` and `WrongType`; bounded accepted-family validation is enforced by [`validate_profile()`](../../../../crates/scriptcraft-core/src/lib.rs:315). |
| PDF emits the matching persisted font mapping | **PASS** | [`pdf_font_name()`](../../../../crates/scriptcraft-core/src/exports.rs:129) maps `Courier` to `Courier` and supported alternatives deterministically; PDF tests assert default `/BaseFont /Courier` and the `Courier Prime` round trip/mapping. |
| Requirements classification | **PASS** | The work order classifies requirements as `verify unchanged`; the inspected requirement contract remains applicable without a new requirement claim. No requirement file was reported as changed. |
| Blueprint/durable-document classification | **PASS WITH NOTE** | The implementation evidence reports the authorized four documentation updates and the inspected blueprints now record the approved policy. The design review's remaining handoff warning that the desktop blueprint was not named in the pre-dispatch authorized list is noted, but the reconciled WO explicitly authorizes it when the desktop font contract is recorded. |
| Required Cargo gates | **PASS by recorded evidence** | The execution record and work order report passing [`cargo fmt --all -- --check`](../../../../Cargo.toml:1), [`cargo check --workspace`](../../../../Cargo.toml:1), [`cargo test --workspace`](../../../../Cargo.toml:1), [`cargo clippy --workspace --all-targets -- -D warnings`](../../../../Cargo.toml:1), [`cargo build --workspace`](../../../../Cargo.toml:1), and [`git diff --check`](../../../../Cargo.toml:1). The recorded test result is 76 tests plus 0 doc tests. These checks were inspected rather than rerun in this tool context. |
| Required project-policy/manual gate | **PASS / no hard stop** | [`project-manifest.md`](../../../project-manifest.md:88) configures no manual/external hard-stop gate. Native visual inspection was not performed and is honestly classified as optional/unavailable in the work order and execution log. |
| Rogue or unrelated behavior | **PASS** | No deceptive, backdoor, malicious, nefarious, or clearly unrelated behavior was identified. The implementation files and behavior are subordinate to the authorized layout/profile/PDF contract. |

## Warnings and notes

### W-001 — Canonical validation filename is occupied by historical v1 evidence

- **Severity:** warning
- **Evidence:** [`validation-WO-001.md`](validation-WO-001.md:1) is the prior v1 Vertical Edge Clamping validation, while the v2 work order metadata points to that same filename at [`WO-001-role-spacing-and-courier-default.md`](../work-orders/WO-001-role-spacing-and-courier-default.md:9). The user explicitly instructed this validator not to overwrite it.
- **Impact:** This is an artifact/link ambiguity, not an implementation-alignment blocker. The fresh v2 result is preserved in [`validation-WO-001-v2.md`](validation-WO-001-v2.md:1), but the work order's validation link does not identify this file.
- **Recommendation:** The orchestrator should resolve the naming/link conflict without deleting the historical v1 record: either rename/archive the historical artifact through an explicitly logged Forge artifact correction, or update the v2 work-order link to this v2 validation filename.
- **Required next state:** Resolve the artifact link before closeout; do not treat the historical v1 file as v2 validation evidence.

### W-002 — No standalone diff was available to this validator

- **Severity:** warning
- **Evidence:** No direct Git diff command/result was available through the inspection tools. The work-order changed-file record, execution log, source, tests, and durable-document evidence are internally consistent and support meaningful comparison.
- **Recommendation:** Preserve the recorded changed-file boundary and attach a comparable branch/file diff if the orchestrator's normal validation environment provides one.
- **Required next state:** Note only; no fix work order is required because comparison was meaningful.

### W-003 — Native visual inspection unavailable

- **Severity:** warning
- **Evidence:** The work order explicitly makes headless layout/profile/PDF checks required and native visual inspection optional; the execution log states native visual inspection was not performed. [`project-manifest.md`](../../../project-manifest.md:90) configures no manual/external hard-stop gate.
- **Recommendation:** Retain the limitation as unavailable; do not infer native screen/PDF visual success from structural tests.
- **Required next state:** Note only; no fix work order is required.

### N-001 — Reconciled review metadata retains stale top-level blocked text

- **Severity:** note
- **Evidence:** [`design-review-v2.md`](../reviews/design-review-v2.md:5) retains an earlier `BLOCKED` status, while its reconciliation verdict at [`design-review-v2.md`](../reviews/design-review-v2.md:207) is `PASS WITH WARNINGS` and the v2 plan records `review → execute` and human approval at [`editor-geometry-corrections-v2.md`](../editor-geometry-corrections-v2.md:21).
- **Required next state:** Documentation cleanup/follow-up only; the reconciled handoff and recorded approval are the applicable v2 execution evidence.

## Decision

- **Outcome:** `pass`
- **Verdict:** `PASS WITH WARNINGS`
- **Implementation alignment:** The inspected implementation satisfies the explicit WO-001 spacing, profile compatibility, PDF mapping, deterministic evidence, scope, and durable-document classification boundaries. No genuine hard-stop finding was identified.
- **Required next state:** The orchestrator may set WO-001 to `complete` under [`05-validation.md`](../../../process/05-validation.md:79), after resolving or explicitly recording the v1/v2 validation-artifact link conflict. Do not create a fix work order. Preserve W-002 and W-003 as evidence limitations, and do not mark later v2 work orders complete or validated from this result.
- **Fix work order required:** `no`

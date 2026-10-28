# Validation: WO-001 — Toolbar Extraction

## Validation metadata

- **Review type:** `validation`
- **Status:** `complete` — independent finding finalized and WO status reconciled to `complete` by the orchestrator.
- **Validator:** Independent fresh-context validator; not the WO-001 implementer or commit/bookkeeping agent.
- **Validator session/agent ID:** Host internal session ID is unavailable in this context; none is inferred.
- **Host dispatch ID:** `01a1834f-7f3f-709c-8286-06098a01a1a9` (actual fresh independent validator host dispatch ID; not an internal session/agent ID).
- **Session exception:** `none`
- **Date:** 2026-10-28
- **Initiative:** [`status-bar-v1.md`](../status-bar-v1.md)
- **Validation file:** `validation-WO-001.md`
- **Validated work order:** [`WO-001-toolbar-extraction.md`](../work-orders/WO-001-toolbar-extraction.md)
- **Compared commit/diff:** `83621f34f5db17b3060b7c28fce795decd903581..fde481db08ba058a1327e2d23abb9b056c2556c3`; target commit `fde481db08ba058a1327e2d23abb9b056c2556c3` has direct parent `83621f34f5db17b3060b7c28fce795decd903581`.
- **Fix chain:** `none`

## Authorized scope and comparison basis

WO-001 authorizes extraction of existing desktop toolbar composition, rendering/allocation helpers, and app adapters into a discrete desktop-owned module, preserving toolbar order, sizing, overflow execution, action/role routing, hover/cursor/focus ownership, and clipped-edge behavior. Status-bar/page behavior and zoom are explicitly deferred. The plan, design review, and approved boundary are recorded in [`status-bar-v1.md`](../status-bar-v1.md), [`design-review.md`](../reviews/design-review.md), and [`WO-001-toolbar-extraction.md`](../work-orders/WO-001-toolbar-extraction.md).

Live Git comparison:

- Branch: `dev/2026-10-28-status-bar`.
- `HEAD` equals the requested target `fde481db08ba058a1327e2d23abb9b056c2556c3`; its direct parent is the requested base `83621f34f5db17b3060b7c28fce795decd903581`.
- Index is empty. The only existing worktree changes at validation start were the two post-commit documentation updates to [`execution-log.md`](../execution-log.md) and [`WO-001-toolbar-extraction.md`](../work-orders/WO-001-toolbar-extraction.md), which record the commit hash/base. They are outside the committed comparison. No implementation/code changes existed in the worktree; a final check after command execution confirmed none.
- The committed range contains 11 paths: additions [`execution-log.md`](../execution-log.md), [`reviews/.gitkeep`](../reviews/.gitkeep), [`design-review.md`](../reviews/design-review.md), [`status-bar-v1.md`](../status-bar-v1.md), [`validations/.gitkeep`](../validations/.gitkeep), [`work-orders/.gitkeep`](../work-orders/.gitkeep), [`WO-001-toolbar-extraction.md`](../work-orders/WO-001-toolbar-extraction.md), [`WO-002-status-items-and-viewport-page.md`](../work-orders/WO-002-status-items-and-viewport-page.md), [`WO-003-transient-zoom-and-integrated-regression.md`](../work-orders/WO-003-transient-zoom-and-integrated-regression.md), plus modified [`main.rs`](../../../../apps/scriptcraft/src/main.rs) and added [`toolbar.rs`](../../../../apps/scriptcraft/src/toolbar.rs).
- The nine Forge paths are initiative planning/review/work-order artifacts for this same approved initiative, not added product behavior. The exact eleven-path commit candidate set and committed path inventory are recorded in [`execution-log.md`](../execution-log.md). The implementation source diff is limited to the two authorized app files; no core crate, Cargo manifest/lock, requirements, blueprint, schema, or unrelated app path changed.

## Acceptance-by-acceptance findings

- **AC-WRITING-005.1 — PASS.** [`default_toolbar_items()`](../../../../apps/scriptcraft/src/toolbar.rs:117) retains the typed eight-item New/Open/Save/Save As/Flexible Space/Elements/Flexible Space/Export PDF composition. [`default_toolbar_has_approved_order_and_labels()`](../../../../apps/scriptcraft/src/main.rs:1458) asserts the exact order and types.
- **AC-WRITING-005.2 — PASS.** [`dispatch_toolbar_action()`](../../../../apps/scriptcraft/src/toolbar.rs:469) routes existing actions to the existing app adapters; [`change_role_from_toolbar()`](../../../../apps/scriptcraft/src/toolbar.rs:556) uses the transactional session operation and app mutation/error path. The overflow handler dispatches selected actions/roles through those same adapters at [`toolbar_overflow_menu()`](../../../../apps/scriptcraft/src/toolbar.rs:638). Regression evidence includes [`toolbar_new_dispatch_replaces_session_without_persisting_path()`](../../../../apps/scriptcraft/src/main.rs:1973), [`overflow_entries_route_actions_and_elements_submenu_choices()`](../../../../apps/scriptcraft/src/main.rs:1936), and the role success/rejection-preservation tests at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:2016) and [`main.rs`](../../../../apps/scriptcraft/src/main.rs:2040). The code paths preserve existing session, persistence, status/error, and fixed-layout export boundaries.
- **AC-WRITING-005.3 — PASS.** [`toolbar_action_button()`](../../../../apps/scriptcraft/src/toolbar.rs:495) creates the click/hover response only for the icon-plus-padding region and paints the centered text label separately, with no added tooltip. Hover-region and label-separation coverage is present in [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1258), [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1270), and [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1346).
- **AC-WRITING-005.4 — PASS.** [`focused_role()`](../../../../apps/scriptcraft/src/toolbar.rs:548) reads the current focused element role; [`elements_toolbar_item()`](../../../../apps/scriptcraft/src/toolbar.rs:561) presents the seven supported role labels and routes selection through the session adapter. Tests cover all role labels, focused-element role selection, and preservation/reporting on rejection at [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1999), [`main.rs`](../../../../apps/scriptcraft/src/main.rs:2016), and [`main.rs`](../../../../apps/scriptcraft/src/main.rs:2040).
- **AC-WRITING-005.5 — PASS.** [`toolbar_item_width()`](../../../../apps/scriptcraft/src/toolbar.rs:142), [`toolbar_content_width()`](../../../../apps/scriptcraft/src/toolbar.rs:173), and [`allocate_toolbar()`](../../../../apps/scriptcraft/src/toolbar.rs:350) preserve measured active-font label widths, typed 72-point Space, fixed width overrides, row padding/gap accounting, equal flexible-space allocation, leftmost-fit narrow overflow, reserved far-right menu, and clipped-content edge budget. Tests cover wide/exact/narrow allocation, widths/overrides, equal flexible space, overflow, and final clipped edge in [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1586), [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1617), [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1675), [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1826), [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1884), and [`main.rs`](../../../../apps/scriptcraft/src/main.rs:1936). The overflow UI routes both actions and Elements roles through existing handlers. No layout state is persisted.
- **AC-WRITING-005.6 — NOT APPLICABLE TO WO-001.** This work order does not authorize title-strip behavior; the source diff leaves the existing title-strip implementation untouched. This is outside this extraction-only acceptance boundary, not a claim of new title-strip validation.
- **AC-WRITING-005.7 — PASS.** Toolbar controls request the standard arrow cursor in [`toolbar_cursor_icon()`](../../../../apps/scriptcraft/src/toolbar.rs:780), while the existing document response requests the text cursor in [`document_cursor_icon()`](../../../../apps/scriptcraft/src/main.rs:1037). Focused cursor-owner tests include [`cursor_intents_are_arrow_for_controls_and_text_for_document_surface()`](../../../../apps/scriptcraft/src/main.rs:1303), [`elements_cursor_ledger_keeps_closed_regions_on_arrow_and_document_on_text()`](../../../../apps/scriptcraft/src/main.rs:1310), and [`cursor_owner_geometry_covers_elements_label_and_document_surface_only()`](../../../../apps/scriptcraft/src/main.rs:1359).
- **NFR-WRITING-001 and NFR-WRITING-002 — NO NEW CORE PATH; NO CONTRADICTION FOUND.** The change relocates desktop toolbar behavior only; it introduces no unsafe code, core editing/layout/persistence path, or new panic-prone input boundary. Workspace tests and strict Clippy pass below. Core remains unchanged.

The toolbar tests are deterministic and exercise the allocation/geometry rules and the app/session adapter behavior. The narrow-focus suite does not simulate actual native pointer/keyboard input; that boundary is reported honestly below rather than inferred as visually tested.

## Required command results

Commands were rerun against live `HEAD` at the requested target commit. All outcomes below are the validator's fresh results, not implementer-reported results.

| Command | Fresh outcome |
|---|---|
| `cargo fmt --all -- --check` | Exit 0; passed. |
| `cargo test -p scriptcraft toolbar_` | Exit 0; 28 passed, 0 failed, 49 filtered out. |
| `cargo check --workspace` | Exit 0; passed with no warnings. |
| `cargo test --workspace` | Exit 0; 159 passed, 0 failed (77 app unit, 45 core unit, 37 core integration; 0 doc tests). |
| `cargo build --workspace` | Exit 0; passed. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Exit 0; passed. |
| `git diff --check 83621f34f5db17b3060b7c28fce795decd903581..fde481db08ba058a1327e2d23abb9b056c2556c3` | Exit 0; committed comparison has no whitespace errors. |
| `git diff --check` | Exit 0; current unstaged recordkeeping edits have no whitespace errors. |
| `cargo run -p scriptcraft` (bounded launch attempt) | Cargo launched `target/debug/scriptcraft`; process remained running for 8 seconds with no startup error/output, then the validator terminated its own process group with SIGTERM (`-15`) to keep the check bounded. This is not counted as a normal-exit run or a visual/input smoke pass. |

## Scope, regression, and limitations

- No status-bar, page-indicator, zoom, horizontal-navigation, core, schema, export, dependency, or unrelated behavior was added. The implementation is a desktop toolbar extraction and wiring change consistent with the initiative plan and desktop ownership contract in [`scriptcraft-desktop.md`](../../../blueprints/containers/scriptcraft-desktop.md); [`screenplay-writing-loop.md`](../../../requirements/features/screenplay-writing-loop.md), the desktop blueprint, feature blueprint, core blueprint, and technical-context blueprint are unchanged in the compared range.
- No deceptive, backdoor, malicious, nefarious, or otherwise rogue behavior was found. No potential code regression was found in the committed extraction; the relevant focused and full regression suites pass.
- **Optional native visual/input smoke limitation:** The bounded process launch did not expose an inspectable native-window/input automation path in this validator surface. Normal/narrow rendering, pointer hit testing, focus, and real native actions therefore remain unobserved; this is not claimed as passed. The initiative and work order classify desktop smoke as recommended rather than a configured hard-stop gate, so this limitation does not block validation.
- **Commit inventory reconciliation note:** WO completion evidence lists the implementation source and two edited evidence files; the commit additionally contains the nine already prepared, same-initiative Forge artifacts listed above. The execution log's explicit candidate and committed-path inventory reconciles these as related planning/review artifacts; no unrelated product source path is present.
- Requirements impact `verify unchanged` and blueprint impact `verify unchanged` are consistent with the relocation-only diff. No durable requirement or blueprint update is needed for WO-001.

## Findings

### Blockers

- None. All explicit WO-001 acceptance criteria applicable to this extraction pass, all required automated/project checks passed, comparison is meaningful, and no unrelated or rogue code was found.

### Warnings

- **Native visual/input smoke unavailable — Severity: warning.** Evidence: only a bounded `cargo run -p scriptcraft` launch was observable; no native-window/input automation was available to inspect normal/narrow rendering or exercise live interactions. Recommendation: retain as unverified optional smoke evidence; do not represent it as a pass. Required next state: `complete` (no hard-stop gate is configured).

### Notes

- The nine initiative-document additions in the implementation commit are related planning/review/work-order artifacts, explicitly inventoried in the execution log, rather than source-scope expansion.
- No WO-002/WO-003 work was started. The work order remains `in-review`; only the orchestrator may reconcile its transition to `complete` after this result is received.

## Decision

- **Outcome:** `pass`
- **Required next state:** Orchestrator may set WO-001 to `complete` after reconciling this validation result; WO status is intentionally unchanged here.
- **Fix work order required:** `no`

## Orchestrator reconciliation — 2026-10-28

- **Validator dispatch:** Reconciled as `01a1834f-7f3f-709c-8286-06098a01a1a9`, the actual host dispatch ID for the fresh independent validator. The host internal session/agent ID is unavailable and is not claimed.
- **State transition:** WO-001 `in-review` → `complete`, after confirming this independent `pass` applies to target commit `fde481db08ba058a1327e2d23abb9b056c2556c3` directly based on `83621f34f5db17b3060b7c28fce795decd903581`.
- **Gate/result:** Applicable acceptance criteria and required automated/project checks passed; no blocker was reported. Native visual/input smoke remains an unavailable optional warning, not a pass or configured hard-stop gate.

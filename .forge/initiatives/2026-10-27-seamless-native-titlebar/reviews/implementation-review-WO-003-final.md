# Fresh Independent Implementation Review — WO-003

## Disposition

**PASS — independent review only.** The narrow initiative-plan correction resolves the prior review blocker. The reviewed cleanup matches approved WO-003 scope and preserves the recorded WO-002b restoration attribution and earlier work-order history. This is not validation, test evidence, or completion approval. WO-003 remains `in-progress`; the separately dispatched fresh validator remains the next gate.

Prior reports [`implementation-review-WO-003.md`](implementation-review-WO-003.md), [`implementation-review-WO-003-followup.md`](implementation-review-WO-003-followup.md), and [`implementation-review-WO-003-reconciled.md`](implementation-review-WO-003-reconciled.md) remain historical and were not edited or replaced.

## Fresh Git snapshot and attribution

Snapshot taken before creating this report:

- **Branch:** `dev/2026-10-27-seamless-native-titlebar`.
- **HEAD:** `da651983a9bdc87439237f032700bbcf4fa1981a`.
- **Staged:** empty.
- **Tracked working-tree paths:** six modified—[`execution-log.md`](../execution-log.md), [`seamless-native-titlebar-v1.md`](../seamless-native-titlebar-v1.md), [`WO-002a-native-transition-scaling-mitigation.md`](../work-orders/WO-002a-native-transition-scaling-mitigation.md), [`Cargo.lock`](../../../../Cargo.lock), [`apps/scriptcraft/Cargo.toml`](../../../../apps/scriptcraft/Cargo.toml), and [`apps/scriptcraft/src/main.rs`](../../../../apps/scriptcraft/src/main.rs)—plus deleted [`macos_titlebar.rs`](../../../../apps/scriptcraft/src/platform/macos_titlebar.rs).
- **Untracked paths:** the three historical review reports listed above, [`WO-002b-restore-native-macos-titlebar.md`](../work-orders/WO-002b-restore-native-macos-titlebar.md), and [`WO-003-titlebar-experiment-residue-cleanup.md`](../work-orders/WO-003-titlebar-experiment-residue-cleanup.md). This report is the only additional artifact created by this review.
- **Diff totals:** seven tracked paths, 201 insertions and 1,472 deletions; no staged paths. The whitespace diff check exited 0.

The WO-003 fresh preflight recorded branch/HEAD and an empty staged diff matching this snapshot; its pre-existing unstaged set was the execution log, initiative plan, WO-002a record, and `main.rs`, with WO-002b and the WO-003 draft untracked ([`WO-003 preflight`](../work-orders/WO-003-titlebar-experiment-residue-cleanup.md:23)). The current additional product paths—the app manifest, lockfile, and bridge deletion—are explicitly attributed to cleanup in the WO-003 implementation record ([`implementation attribution`](../work-orders/WO-003-titlebar-experiment-residue-cleanup.md:100)). `main.rs` remains cumulative: the earlier native-header restoration is not attributed wholesale to WO-003. The plan, execution log, WO-002a record, and untracked WO-002b order retain their previously recorded restoration/history attribution. No reset, staging, or commit occurred.

## Initiative-plan reconciliation and historical record

The authorized current-state summary at [`seamless-native-titlebar-v1.md`](../seamless-native-titlebar-v1.md:6) says WO-003 is `in-progress` with review and validation pending, WO-002 remains `blocked` with its original `fail`, and WO-002a remains `blocked`; it does not claim completion. The two unauthorized WO-003-specific approval/dispatch and implementation-results appendices identified by the prior reconciled review are absent from the current plan. No WO-003 approval/implementation appendix remains. The pre-existing dated restoration and sequencing history remains in place, including the work-order sequence and WO-002b user-gate history ([`plan history`](../seamless-native-titlebar-v1.md:215), [`restoration amendment`](../seamless-native-titlebar-v1.md:227), [`prior restoration sequencing`](../seamless-native-titlebar-v1.md:338)). The former scaling draft's WO-003 numbering remains historical, not a second current cleanup order.

The execution log and WO-003 retain the reconciliation authorization and correction record ([`WO-003 reconciliation record`](../work-orders/WO-003-titlebar-experiment-residue-cleanup.md:117), [`scope correction`](../work-orders/WO-003-titlebar-experiment-residue-cleanup.md:122)). Existing review reports are still separate, untracked historical artifacts and were not rewritten.

## Cleanup, dependency, and protected-behavior review

- **Bridge and direct-use audit:** The deleted AppKit bridge contained the [`performZoom:`](../work-orders/WO-003-titlebar-experiment-residue-cleanup.md:37) experiment and its bridge-only native-control hit-region test. A fresh recursive source/configuration search returned no remaining bridge symbols, module inclusion, or direct uses of `raw_window_handle`, `objc2`, or `objc2_app_kit` in app/core source and manifests. The bridge and its test are removed together; no replacement native-window action was introduced.
- **Cleanup boundary:** The source no longer contains the retired custom/hidden title-strip mode, app-owned maximize/transition policy, experiment-only viewport/chrome ledgers, or their writer plumbing and tests. Search for the retired symbols returned no matches. General performance/input diagnostics and current page-fit behavior remain. The cleanup classification and protected boundaries are specified in [`WO-003 candidate table`](../work-orders/WO-003-titlebar-experiment-residue-cleanup.md:35) and [`protected behavior`](../work-orders/WO-003-titlebar-experiment-residue-cleanup.md:59).
- **Dependencies and lockfile:** [`apps/scriptcraft/Cargo.toml`](../../../../apps/scriptcraft/Cargo.toml:10) removes only the macOS direct declarations for `raw-window-handle`, `objc2`, and `objc2-app-kit`; the package-local unsafe-code and Clippy policies remain. The [`scriptcraft` lock entry](../../../../Cargo.lock:2698) loses only those three direct edges. A read-only macOS target dependency-tree inspection confirms `raw-window-handle 0.6.2`, `objc2 0.6.5`, and `objc2-app-kit 0.3.2` remain transitively required through the GUI/windowing and dialog dependencies; their lockfile package records and versions remain.
- **Native startup and toolbar:** Current startup retains the ordinary visible, decorated, opaque native window tuple with [`fullsize_content_view(false)`](../../../../apps/scriptcraft/src/main.rs:1935) and no app-owned maximize/restore substitute. Retained focused assertions cover the native startup tuple and toolbar content boundary/full width/height, fill/style, restored bottom padding, and clear-color ownership ([`native startup test`](../../../../apps/scriptcraft/src/main.rs:1983), [`toolbar geometry test`](../../../../apps/scriptcraft/src/main.rs:2013), [`toolbar fill test`](../../../../apps/scriptcraft/src/main.rs:2027), [`toolbar padding test`](../../../../apps/scriptcraft/src/main.rs:2195), [`clear-color test`](../../../../apps/scriptcraft/src/main.rs:2219)). General performance and native-input diagnostics remain in the current source.
- **Scope/attribution conclusion:** The deletions are the identified experiment residue; no unrelated product/core paths or historical work-order/validation paths appear in the tracked diff. The pre-existing WO-002b native-header restoration changes remain preserved and are not reattributed to WO-003.

## Historical dispositions and verification boundary

- WO-002 remains `blocked`, and [`validation-WO-002.md`](../validations/validation-WO-002.md:51) remains `fail`.
- WO-002a remains `blocked` after the unsuccessful renewed attempt ([`WO-002a disposition`](../work-orders/WO-002a-native-transition-scaling-mitigation.md:122)). WO-002b remains at its recorded implementation status; its user-reported visual-gate pass is not represented as a validator result or completion.
- No tests, builds, formatters, or validation commands were run in this review. The dependency-tree inspection and `git diff --check` are review evidence only; implementation-reported test/build results remain attributed to the implementer.
- **Final result: PASS for independent review only.** Keep WO-003 `in-progress`; do not mark it complete or validated. A separate fresh validator follows this review.

# Independent Implementation Review Follow-up — WO-003

## Review scope and disposition

- **Review type:** Fresh, read-only implementation and diff-attribution follow-up. This is not a test run, validation artifact, visual gate, or completion decision.
- **Disposition: BLOCKED — initiative-plan change is outside the expressly listed WO-003 documentation update scope and its current-state summary is inconsistent.** The fresh Git evidence resolves the prior review's diff-access limitation. The product changes inspected are consistent with the cleanup candidates and protected behavior in the approved order; no product-source defect was identified. However, the worktree contains WO-003-attributed changes to the initiative plan, while the approved WO-003 in-scope documentation item names the work order and execution log. The plan also retains a stale header contradicting its later WO-003 approval/implementation record. This requires an explicit scope/authorization decision and an authorized reconciliation before this review can pass.
- **Prior review preserved:** [`implementation-review-WO-003.md`](implementation-review-WO-003.md:1) remains intact. Its disposition was blocked because the reviewer could not access live Git status/diffs and therefore could not independently attribute exact hunks or deletions. That limitation is resolved here by capturing Git directly; the prior report is neither replaced nor erased. The new blocker below is based on evidence now available, not on that earlier tool limitation.
- **State boundary:** Keep WO-003 `in-progress`; do not infer acceptance, validation, or completion. WO-002 remains `blocked` with its existing `fail` validation, WO-002a remains `blocked`, and WO-002b remains at its recorded implementation status. No historical disposition is changed.

## Fresh Git evidence

Snapshot captured read-only on 2026-10-28. No staging, reset, overwrite, commit, merge, source edit, or validation command was performed.

- **Branch/status:** `git status --short --branch` reported `## dev/2026-10-27-seamless-native-titlebar`.
- **HEAD:** `da651983a9bdc87439237f032700bbcf4fa1981a`.
- **Staged:** `git diff --cached --name-status` and `git diff --cached --stat` were empty.
- **Unstaged tracked name-status:**
  - Modified: `.forge/initiatives/2026-10-27-seamless-native-titlebar/execution-log.md`
  - Modified: `.forge/initiatives/2026-10-27-seamless-native-titlebar/seamless-native-titlebar-v1.md`
  - Modified: `.forge/initiatives/2026-10-27-seamless-native-titlebar/work-orders/WO-002a-native-transition-scaling-mitigation.md`
  - Modified: `Cargo.lock`
  - Modified: `apps/scriptcraft/Cargo.toml`
  - Modified: `apps/scriptcraft/src/main.rs`
  - Deleted: `apps/scriptcraft/src/platform/macos_titlebar.rs`
- **Unstaged stat:** 7 tracked paths; 205 insertions and 1,472 deletions.
- **Untracked paths:**
  - `.forge/initiatives/2026-10-27-seamless-native-titlebar/reviews/implementation-review-WO-003.md` (the prior review; preserved)
  - `.forge/initiatives/2026-10-27-seamless-native-titlebar/work-orders/WO-002b-restore-native-macos-titlebar.md`
  - `.forge/initiatives/2026-10-27-seamless-native-titlebar/work-orders/WO-003-titlebar-experiment-residue-cleanup.md`

The freshly observed branch, HEAD, empty staged diff, four pre-existing modified paths, and pre-existing untracked WO-002b/WO-003 artifacts match the implementer preflight recorded in [`WO-003`](../work-orders/WO-003-titlebar-experiment-residue-cleanup.md:17) and its fresh attribution section at lines 23–29. The three additional tracked product paths—app manifest, lockfile, and deleted bridge—are the cleanup changes declared by the WO-003 implementation record. The prior review artifact is an expected untracked item from the review history, not a product change. No unexplained tracked or untracked path was found.

The cumulative `main.rs` diff includes the earlier WO-002b restoration as well as WO-003 cleanup; it must not be attributed wholesale to WO-003. The baseline and restoration evidence say WO-002b restored the visible OS-owned titlebar, disabled full-size content, removed the native 28-point band/dispatch, reverted WO-002a easing, and retained dormant custom/transition code ([`WO-002b evidence`](../work-orders/WO-002b-restore-native-macos-titlebar.md:13), [`WO-002b preservation scope`](../work-orders/WO-002b-restore-native-macos-titlebar.md:26)). The exact current source diff contains that restored native tuple plus the separately listed WO-003 removals. The WO-002a file remains modified as part of the pre-existing restoration/history baseline; it is not listed as a WO-003 cleanup deletion. WO-002 and its validation are absent from the changed-path inventory.

## Product diff and scope findings

### Source, deletion, and dependencies

- **`main.rs`:** The exact diff removes the AppKit module inclusion and call path; native-band hit testing and app-side `performZoom:` dispatch; the titlebar mode selector/custom-strip helpers; app-owned maximize/transition policy; the old top inset; viewport/chrome diagnostic samples, ledgers, and their writer plumbing. It also removes tests whose sole subjects were those deleted mechanisms. The former native startup tuple is simplified to a single tuple with decorations on, opaque presentation, visible title and native buttons, and `fullsize_content_view(false)`, matching the recorded restoration. The selected top panel retains the shared chrome frame, disabled separator, live toolbar metrics, and exact toolbar-content height.
- **Deleted bridge:** `apps/scriptcraft/src/platform/macos_titlebar.rs` is exactly the 101-line bridge and bridge-only hit-region test described by the work order. The module/path inclusion and all searched bridge/API symbols are absent from current app/core Rust sources and manifests.
- **Direct dependencies:** `apps/scriptcraft/Cargo.toml` removes only the macOS-target direct declarations for `raw-window-handle`, `objc2`, and `objc2-app-kit`. The package-local `unsafe_code = deny` policy and Clippy denies remain. The `scriptcraft` entry in `Cargo.lock` loses only those three direct edges; the crate package entries and references from other dependencies remain. A repository source/manifest search found no remaining direct API/import use. The implementation's recorded host/macOS-target Cargo graph and reverse-tree checks are implementer evidence only; they were not rerun in this review.

### Preserved behavior and test coverage

- The current startup test still asserts the single visible-native-header tuple, including normal decorations, visible buttons/titlebar, opaque window, and `fullsize_content_view(false)` ([`native startup test`](../../../../apps/scriptcraft/src/main.rs:1983)).
- The toolbar content-origin/full-width/full-height test remains ([`toolbar boundary test`](../../../../apps/scriptcraft/src/main.rs:2013)); focused fill, exact panel-padding, and clear-color tests also remain ([`toolbar fill test`](../../../../apps/scriptcraft/src/main.rs:2027), [`toolbar padding test`](../../../../apps/scriptcraft/src/main.rs:2195), [`clear-color test`](../../../../apps/scriptcraft/src/main.rs:2219)). The removed bottom-padding ledger assertion is replaced by a direct assertion over the current toolbar row and panel padding span.
- Current page-fit metrics, toolbar allocation/actions/Elements behavior, cursor behavior, keyboard/IME, editing, save/open, and general `PerformanceDiagnostics`/`NativeInputDiagnostics` code remain visible in the current source. The tests removed are confined to the retired title strip, transition policy, diagnostic ledgers, or bridge-only behavior; the implementation record reports no broad test pruning.
- The app/core search for retired symbols and direct dependency names returned no matches. Cargo.lock retains transitive packages, consistent with the work-order condition that only direct edges be removed. These are static review observations, not fresh build/test/dependency-graph validation.

The implementation claims and exact product diff align with the approved remove/retain classifications in [`WO-003`](../work-orders/WO-003-titlebar-experiment-residue-cleanup.md:31) and its implementation evidence at lines 100–108. This review identified no application behavior change in the selected native path and did not rerun or claim any tests/checks. Recorded implementation checks remain attributed to the implementer; the separate validator owns fresh validation.

## Blocker: initiative-plan scope and current-state inconsistency

The exact diff for [`seamless-native-titlebar-v1.md`](../seamless-native-titlebar-v1.md:1) adds/changes the WO-003 approval/dispatch and implementation-results sections, and the WO-003 implementation record explicitly lists the initiative plan among its attributed paths ([`WO-003 implementation attribution`](../work-orders/WO-003-titlebar-experiment-residue-cleanup.md:107); [`plan implementation entry`](../seamless-native-titlebar-v1.md:348)). However, the approved work order's in-scope documentation instruction specifically authorizes updating **this work order and the execution log** ([`WO-003 scope`](../work-orders/WO-003-titlebar-experiment-residue-cleanup.md:49), especially line 57); it does not expressly authorize editing the initiative plan. The WO-003 preflight confirms that the plan already had pre-existing WO-002b/restoration changes, but the current plan diff also contains later WO-003 approval/implementation material. No separate authorization extending this order's documentation scope is present in the reviewed evidence.

The plan's top status summary still says WO-003 is awaiting approval and that no cleanup implementation is authorized ([`stale plan summary`](../seamless-native-titlebar-v1.md:5)), while its later WO-003 section records explicit approval and implementation in progress ([`later plan state`](../seamless-native-titlebar-v1.md:341)). This is a documentation inconsistency, not a source defect, but reinforces that the plan change cannot be accepted as an incidental, self-authorizing scope expansion.

**Evidence needed to unblock:** an explicit user authorization for the initiative-plan additions (or a revised approved WO-003 that includes that artifact and defines its update boundary), together with an authorized resolution of the contradictory plan summary. No such artifact was created or edited by this review. Until that scope and state discrepancy is resolved, the review remains **BLOCKED**.

## Historical and validation boundaries

- [`WO-002`](../work-orders/WO-002-native-titlebar-double-click.md:3) remains blocked and [`validation-WO-002.md`](../validations/validation-WO-002.md:49) retains its `fail` outcome. [`WO-002a`](../work-orders/WO-002a-native-transition-scaling-mitigation.md:3) remains blocked. WO-002b remains at its recorded implementation status; its user-reported visual gate is not converted into a validator result or completion claim.
- No tests, builds, formatters, dependency-graph commands, or validation commands were run for this follow-up. No result is claimed beyond the static Git/source/document review above.
- No product source, test, manifest, lockfile, work order, validation, initiative plan, or execution log was changed by the reviewer. Only this separate follow-up report was added. WO-003 is not marked complete.
